//! Recipe executes one model graph after automatically probing a compiled discrete GPU backend.
//! Attention is three-projection scaled Q/K/V without an output projection.
#![allow(non_upper_case_globals)]
mod bundle {
	use super::*;
	use std::{collections::BTreeMap, io::Write as _, str::FromStr};
	#[derive(Clone)]
	pub(super) struct StoredGraph {
		pub graph: Graph, pub precision: FloatFormat, pub inputs: Vec<String>, pub outputs: Vec<String>,
		pub norm_mean: Vec<f64>, pub norm_scale: Vec<f64>, pub target_min: f64, pub target_span: f64, pub bn_stats: Vec<f64>,
	}
	#[derive(Default)]
	struct Builder {
		inputs: Vec<String>, outputs: Vec<String>, input: Option<Shape>, output: Option<Shape>, nodes: Vec<Node>, arguments: usize,
		parameters: Vec<f64>, frozen: Vec<u8>, programs: Vec<f64>, state: TrainingState, precision: Option<FloatFormat>,
		norm_mean: Vec<f64>, norm_scale: Vec<f64>, target_min: f64, target_span: f64, bn_stats: Vec<f64>,
	}
	impl Builder {
		fn finish(self) -> Result<StoredGraph> {
			let (input, output) = (self.input.ok_or_else(|| RecipeError::new("model graph has no input shape"))?, self.output.ok_or_else(|| RecipeError::new("model graph has no output shape"))?);
			require(!self.nodes.is_empty(), "model graph has no nodes")?;
			require(self.arguments == self.nodes.len(), "model graph node arguments are incomplete")?;
			require(self.parameters.len() == self.frozen.len(), "model graph frozen weights are incomplete")?;
			for (name, values) in [("moments", &self.state.moments), ("variances", &self.state.variances), ("best weights", &self.state.best)] {
				require(values.is_empty() || values.len() == self.parameters.len(), format!("model graph {name} are incomplete"))?;
			}
			require(self.state.best_loss.is_empty() || self.state.best_loss.len() == 4, "model graph best loss state is incomplete")?;
			require(self.inputs.len() == input.elements(), "model graph input schema has the wrong width")?;
			require(self.outputs.len() == output.elements(), "model graph output schema has the wrong width")?;
			require(self.norm_mean.len() == self.norm_scale.len() && (self.norm_mean.is_empty() || self.norm_mean.len() == self.inputs.len()), "model graph normalization stats have the wrong width")?;
			let source = self.nodes.len() as i32 - 1;
			Ok(StoredGraph { graph: Graph { nodes: self.nodes, parameters: self.parameters, frozen: self.frozen, programs: self.programs, input, output, source, state: self.state, block_index: 0, block_kind: "" }, precision: self.precision.ok_or_else(|| RecipeError::new("model graph has no arithmetic format"))?, inputs: self.inputs, outputs: self.outputs, norm_mean: self.norm_mean, norm_scale: self.norm_scale, target_min: self.target_min, target_span: self.target_span, bn_stats: self.bn_stats })
		}
	}
	fn values<T: FromStr>(text: &str, role: &str) -> Result<Vec<T>>
	where
		T::Err: fmt::Display,
	{
		text.split_whitespace().map(|value| value.parse().map_err(|error| RecipeError::new(format!("invalid {role}: {error}")))).collect()
	}
	fn value<T: FromStr>(text: &str, role: &str) -> Result<T>
	where
		T::Err: fmt::Display,
	{
		text.parse().map_err(|error| RecipeError::new(format!("invalid {role}: {error}")))
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
		Ok(Node { op: primitive(value[0])?, source: value[1], second: value[2], input: Shape { channels: value[3] as usize, length: value[4] as usize }, output: Shape { channels: value[5] as usize, length: value[6] as usize }, offset: value[7] as usize, parameters: value[8] as usize, argument: [0.0; 9], program_offset: value[9] as usize, program_count: value[10] as usize, block_index: 0, block_kind: "" })
	}
	fn precision(value: &str) -> Result<FloatFormat> {
		let fields = value.split_whitespace().collect::<Vec<_>>();
		require(fields.len() == 5, "arithmetic format has the wrong width")?;
		let shape = (fields[0], self::value::<u8>(fields[1], "arithmetic bits")?, self::value::<u8>(fields[2], "arithmetic exponent")?, self::value::<u8>(fields[3], "arithmetic mantissa")?, fields[4]);
		match shape { ("fp", 64, 11, 52, "double") => Ok(FP64), ("fp", 32, 8, 23, "float") => Ok(FP32), ("fp", 16, 5, 10, "half") => Ok(FP16), ("f", 64, 11, 52, "double") => Ok(FloatFormat { family: "f", ..FP64 }), ("f", 32, 8, 23, "float") => Ok(FloatFormat { family: "f", ..FP32 }), ("f", 16, 5, 10, "half") => Ok(FloatFormat { family: "f", ..FP16 }), _ => Err(RecipeError::new(format!("saved arithmetic format {}({}) [{}] is unavailable; available precision: f(5,10) [half], f(8,23) [float], f(11,52) [double], fp(16) [half], fp(32) [float], fp(64) [double]", fields[0], fields[1], fields[4]))) }
	}
	pub(super) fn load(path: &str) -> Result<(String, Vec<StoredGraph>)> {
		require(path.ends_with(".ogdl"), "model path requires .ogdl")?;
		let document = fs::read_to_string(path).map_err(|error| RecipeError::new(format!("cannot read {path}: {error}")))?;
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
				"arithmetic" => builder.precision = Some(precision(value)?),
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
					builder.nodes.last_mut().ok_or_else(|| RecipeError::new("argument precedes node"))?.argument.copy_from_slice(&argument);
					builder.arguments += 1;
				}
				"quantization" => {}
				"programs" => builder.programs = values(value, "scalar program")?,
				"norm_mean" => builder.norm_mean = values(value, "normalization mean")?,
				"norm_scale" => builder.norm_scale = values(value, "normalization scale")?,
				"target_min" => builder.target_min = self::value(value, "target min")?,
				"target_span" => builder.target_span = self::value(value, "target span")?,
				"bn_stats" => builder.bn_stats = values(value, "batch norm stats")?,
				"weights" => builder.parameters.extend(values::<f64>(value, "weight")?),
				"quantized" => {
					let mut fields = value.split_whitespace();
					let code = self::value(fields.next().unwrap_or(""), "quantization code")?;
					let count = self::value(fields.next().unwrap_or(""), "quantized weight count")?;
					let codebook = fields.next().unwrap_or("");
					let codebook = if codebook == "-" { Vec::new() } else { codebook.split(',').map(|value| self::value(value, "quantization codebook")).collect::<Result<Vec<_>>>()? };
					let hex = fields.next().unwrap_or("");
					require(hex.len() % 2 == 0, "quantized bytes are invalid")?;
					let bytes = (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|error| RecipeError::new(format!("invalid quantized byte: {error}")))).collect::<Result<Vec<_>>>()?;
					builder.parameters.extend(IntegerFormat(code).decompress(&bytes, &codebook, count)?);
				}
				"frozen" => builder.frozen = values(value, "frozen weight")?,
				"moments" => builder.state.moments = values(value, "Adam moment")?,
				"variances" => builder.state.variances = values(value, "Adam variance")?,
				"best" => builder.state.best = values(value, "best weight")?,
				"best_loss" => builder.state.best_loss = values(value, "best loss")?,
				"epoch" => builder.state.epoch = self::value(value, "epoch")?,
				"training_rows" => builder.state.training_rows = self::value(value, "training_rows")?,
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
		let config = Config::load()?;
		for stored in graphs {
			document.push_str("    graph\n");
			for name in &stored.inputs {
				document.push_str(&format!("        in {name}\n"))
			}
			for name in &stored.outputs {
				document.push_str(&format!("        out {name}\n"))
			}
			let graph = &stored.graph;
			field(&mut document, "arithmetic", &format!("{} {} {} {} {}", stored.precision.family, stored.precision.bits, stored.precision.exponent, stored.precision.mantissa, stored.precision.llvm));
			let quantized = graph.nodes.iter().any(|node| node.argument[8] != 0.0);
			field(&mut document, "shape", &format!("{} {} {} {}", graph.input.channels, graph.input.length, graph.output.channels, graph.output.length));
			for node in &graph.nodes {
				let d = [node.op as i32, node.source, node.second, node.input.channels as i32, node.input.length as i32, node.output.channels as i32, node.output.length as i32, node.offset as i32, node.parameters as i32, node.program_offset as i32, node.program_count as i32];
				field(&mut document, "node", &join(&d));
				field(&mut document, "   arguments", &join(&node.argument));
				if node.argument[8] != 0.0 {
					field(&mut document, "   quantization", &quantization(node.argument[8] as u16))
				}
			}
			for node in &graph.nodes {
				if node.parameters != 0 {
					let weights = &graph.parameters[node.offset..node.offset + node.parameters];
					if node.argument[8] == 0.0 {
						field(&mut document, "weights", &join(weights))
					} else {
						let format = IntegerFormat(node.argument[8] as u16);
						let importance = graph.state.variances.get(node.offset..node.offset + node.parameters).unwrap_or(&[]); let (bytes, codebook) = format.compress(weights, importance, config)?;
						let hex = bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
						let metadata = if codebook.is_empty() { "-".to_owned() } else { codebook.iter().map(ToString::to_string).collect::<Vec<_>>().join(",") };
						field(&mut document, "quantized", &format!("{} {} {metadata} {hex}", format.0, weights.len()))
					}
				}
			}
			for (key, value) in [("programs", join(&graph.programs)), ("norm_mean", join(&stored.norm_mean)), ("norm_scale", join(&stored.norm_scale)), ("frozen", join(&graph.frozen)), ("moments", (!quantized).then(|| join(&graph.state.moments)).unwrap_or_default()), ("variances", (!quantized).then(|| join(&graph.state.variances)).unwrap_or_default()), ("best", (!quantized).then(|| join(&graph.state.best)).unwrap_or_default()), ("best_loss", (!quantized).then(|| join(&graph.state.best_loss)).unwrap_or_default()), ("epoch", if quantized { "0".to_owned() } else { graph.state.epoch.to_string() }), ("training_rows", graph.state.training_rows.to_string()), ("trained_samples", join(&graph.state.trained_samples))] {
				field(&mut document, key, &value)
			}
			if stored.target_span != 0.0 {
				field(&mut document, "target_min", &stored.target_min.to_string());
				field(&mut document, "target_span", &stored.target_span.to_string());
			}
			if !stored.bn_stats.is_empty() {
				field(&mut document, "bn_stats", &join(&stored.bn_stats))
			}
		}
		fs::write(path, document).map_err(|error| RecipeError::new(format!("cannot write {path}: {error}")))?;
		eprintln!("saved: {path}");
		Ok(())
	}
	fn same_node(a: &Node, b: &Node) -> bool {
		a.op == b.op && a.source == b.source && a.second == b.second && a.input == b.input && a.output == b.output && a.offset == b.offset && a.parameters == b.parameters && a.argument[..8].iter().zip(&b.argument).all(|(a,b)|a.to_bits()==b.to_bits()) && a.program_offset == b.program_offset && a.program_count == b.program_count
	}
	fn same_graph(a: &StoredGraph, b: &StoredGraph) -> bool {
		a.precision == b.precision && a.inputs == b.inputs && a.outputs == b.outputs && a.graph.input == b.graph.input && a.graph.output == b.graph.output && a.graph.frozen == b.graph.frozen && a.graph.programs == b.graph.programs && a.graph.parameters.len() == b.graph.parameters.len() && a.graph.nodes.len() == b.graph.nodes.len() && a.graph.nodes.iter().zip(&b.graph.nodes).all(|(a, b)| same_node(a, b))
	}
	pub(super) fn restore(path: &str, schema: &str, graphs: &mut [StoredGraph], identities: &[u64]) -> Result<()> {
		if !fs::exists(path).map_err(|error| RecipeError::new(format!("cannot inspect {path}: {error}")))? {
			return save(path, schema, graphs);
		}
		let (stored_schema, stored) = load(path)?;
		let matches = stored_schema == schema && stored.len() == graphs.len() && stored.iter().zip(graphs.iter()).all(|(a, b)| same_graph(a, b));
		if matches {
			for (current, saved) in graphs.iter_mut().zip(&stored) {
				let saved_boundary = saved.graph.state.training_rows;
				let current_boundary = current.graph.state.training_rows;
				if saved_boundary != 0 {
					require(!saved.graph.state.trained_samples.is_empty(), "resume rejected: saved model has no training membership identity")?;
					require(current_boundary <= identities.len(), "current training membership is incomplete")?;
					let trained = saved.graph.state.trained_samples.iter().copied().collect::<BTreeSet<_>>();
					let overlap = identities[current_boundary..].iter().filter(|value| trained.contains(value)).count();
					require(overlap == 0, format!("resume rejected: {overlap} evaluation samples were previously trained, current boundary is {current_boundary} and saved boundary was {saved_boundary}"))?;
				}
				let same = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.to_bits() == b.to_bits());
				require(same(&current.norm_mean, &saved.norm_mean) && same(&current.norm_scale, &saved.norm_scale) && current.target_min.to_bits() == saved.target_min.to_bits() && current.target_span.to_bits() == saved.target_span.to_bits(), format!("resume rejected: fitted preprocessing differs, current boundary is {current_boundary} and saved boundary was {saved_boundary}"))?;
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
		std::io::stdin().read_line(&mut answer).map_err(|error| RecipeError::new(format!("cannot read answer: {error}")))?;
		require(answer.trim().is_empty() || answer.trim().eq_ignore_ascii_case("y"), "model mismatch not overwritten")?;
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
	pub(super) fn run_infer(path: &str, input: &[f64], forward: impl Fn(&StoredGraph, &[f64]) -> Result<Vec<f64>>) -> Result<Vec<f64>> {
		let (_, graphs) = load(path)?;
		let first = graphs.first().ok_or_else(|| RecipeError::new("model has no graph"))?;
		require(input.len() == first.inputs.len(), "model input has the wrong width")?;
		let mut values = first.inputs.iter().cloned().zip(input.iter().copied()).collect::<BTreeMap<_, _>>();
		let mut result = Vec::new();
		for stored in graphs {
			let mut samples = stored.inputs.iter().map(|name| values.get(name).copied().ok_or_else(|| RecipeError::new(format!("input {name:?} is absent")))).collect::<Result<Vec<_>>>()?;
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
impl RecipeError { fn new(message: impl Into<String>) -> Self { Self(message.into()) } }
impl fmt::Display for RecipeError { fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result { formatter.write_str(&self.0) } }
impl Error for RecipeError {}
pub type Result<T> = std::result::Result<T, RecipeError>;
type Ptr = *mut c_void;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Backend { Cpu, Amd, Nvidia }
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
struct Route { inputs: Vec<String>, outputs: Vec<String> }
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Residual { Layer(usize), Activation(Activation) }
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
enum BlockNormalization { Batch, Layer }
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
const IQ_DEFAULT: [u16; 5] = [0, 3, 1, 1, 5];
#[derive(Clone, Debug, PartialEq, Eq)]
struct Block {
	operation: Operation, activation: Activation, normalization: Option<BlockNormalization>, format: FloatFormat, quantization: u16, profile: bool,
}
#[derive(Clone)]
pub struct Model {
	blocks: Vec<Block>, loss: LossFunction, downstream: Option<Vec<Block>>, format: FloatFormat, quantization: u16,
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
$(pub fn $method(&self, $($argument: $kind),*) -> Self { self.push($operation) })+ }; }
impl Model {
	fn push(&self, operation: Operation) -> Self {
		let mut model = self.clone();
		model.blocks.push(Block { operation, activation: Activation::Linear, normalization: None, format: model.format, quantization: model.quantization, profile: IntegerFormat(model.quantization).selection().is_some() });
		model
	}
	fn activate(&self, activation: Activation) -> Self {
		let mut model = self.clone();
		let block = model.blocks.last_mut().unwrap_or_else(|| panic!("activation requires a preceding block"));
		if block.normalization.is_some() {
			panic!("activation must precede normalization");
		}
		block.activation = activation;
		model
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
	pub fn residual<const N: usize>(&self, parts: [Residual; N]) -> Self { self.push(Operation::Residual(parts.into())) }
	pub fn moe<const N: usize>(&self, top_k: usize, experts: [Residual; N]) -> Self { self.push(Operation::Moe(top_k, experts.into())) }
	pub fn svm<const N: usize>(&self, choices: [fn() -> Residual; N]) -> Self {
		let choices = choices
			.into_iter()
			.map(|choice| match choice() {
				Residual::Activation(value) => value,
				Residual::Layer(_) => panic!("SVM choices must be activations"),
			})
			.collect();
		self.push(Operation::Svm(choices))
	}
	pub fn norm(&self, normalization: Normalization) -> Self {
		let mut model = self.clone();
		let block = model.blocks.last_mut().unwrap_or_else(|| panic!("normalization requires a preceding block"));
		block.normalization = Some(if normalization as usize == batch as usize { BlockNormalization::Batch } else { BlockNormalization::Layer });
		model
	}
	pub fn loss(&self, loss: impl ModelLoss) -> Self { let mut model = self.clone(); loss.apply(&mut model); model }
	fn arithmetic(&self, format: FloatFormat) -> Self {
		let mut model = self.clone();
		if let Some(block) = model.blocks.last_mut() {
			block.format = format
		} else {
			model.format = format
		}
		model
	}
	pub fn f(&self, exponent: u8, mantissa: u8) -> Self { assert!(exponent != 0 && mantissa != 0, "f requires exponent and mantissa fields"); let llvm = match (exponent, mantissa) { (5, 10) => "half", (8, 23) => "float", (11, 52) => "double", _ => "unsupported" }; self.arithmetic(FloatFormat { family: "f", bits: exponent + mantissa + 1, exponent, mantissa, llvm }) }
	pub fn fp(&self, bits: u8) -> Self {
		let (exponent, mantissa, llvm) = match bits {
			8 => (4, 3, "unsupported"),
			16 => (5, 10, "half"),
			32 => (8, 23, "float"),
			64 => (11, 52, "double"),
			_ => panic!("fp bits must be 8, 16, 32, or 64"),
		};
		self.arithmetic(FloatFormat { family: "fp", bits, exponent, mantissa, llvm })
	}
	pub fn int(&self, bits: u8) -> Self { assert!([1, 4, 8].contains(&bits), "int bits must be 1, 4, or 8"); self.arithmetic(FloatFormat { family: "int", bits, exponent: 0, mantissa: 0, llvm: "unsupported" }) }
	pub fn bf(&self, bits: u8) -> Self { assert_eq!(bits, 16, "bf bits must be 16"); self.arithmetic(FloatFormat { family: "bf", bits, exponent: 8, mantissa: 7, llvm: "bfloat" }) }
	pub fn tf(&self, bits: u8) -> Self { assert_eq!(bits, 32, "tf bits must be 32"); self.arithmetic(FloatFormat { family: "tf", bits, exponent: 8, mantissa: 10, llvm: "unsupported" }) }
	fn quantize(&self, family: u16, bits: u8, variant: u16) -> Self {
		let mut model = self.clone();
		let format = family << 12 | variant << 8 | u16::from(bits);
		if let Some(block) = model.blocks.last_mut() {
			block.quantization = format; block.profile = IntegerFormat(format).selection().is_some()
		} else {
			model.quantization = format
		}
		model
	}
	pub fn qi(&self, bits: u8) -> Qi {
		assert!([2,3,4,5,6,8].contains(&bits), "qi bits must be 2, 3, 4, 5, 6, or 8");
		let q = |v| self.quantize(0, bits, v);
		Qi { model: q(0), zero: q(0), one: q(1), nf: q(2), k: Qk { model: q(3), s: q(4), m: q(5), l: q(6) } }
	}
	pub fn iq(&self, bits: u8) -> Iq {
		assert!((1..=4).contains(&bits), "iq bits must be 1 through 4");
		let q = |v| self.quantize(1, bits, v);
		Iq { model: q(IQ_DEFAULT[usize::from(bits)]), xxs: q(1), xs: q(2), s: q(3), m: q(4), nl: q(5) }
	}
	fn description(&self, metrics: &[Metric]) -> String {
		let has = |value| metrics.iter().any(|metric| metric.0 == value);
		let (operation, activation, normalization) = (has(5), has(6), has(7));
		let output = usize::from(matches!(self.blocks.last(), Some(Block { operation: Operation::Layer(1), activation: Activation::Linear, normalization: None, .. })));
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
fn quantization(code: u16) -> String {
	let (family, bits, variant) = (code >> 12, code as u8, usize::from(code >> 8 & 15));
	let suffix = if family == 0 { ["_0", "_1", "_NF", "_K", "_K_S", "_K_M", "_K_L"][variant] } else { ["", "_XXS", "_XS", "_S", "_M", "_NL"][variant] };
	format!("{}{bits}{suffix}", if family == 0 { "Q" } else { "IQ" })
}
#[rustfmt::skip]
fn fp16(value: f32) -> u16 {
	let bits = value.to_bits();
	let sign = (bits >> 16 & 0x8000) as u16;
	let exponent = ((bits >> 23 & 0xff) as i32) - 112;
	let mantissa = bits & 0x7fffff;
	if exponent <= 0 {
		if exponent < -10 { return sign }
		let value = (mantissa | 0x800000) >> (1 - exponent);
		return sign | ((value + 0xfff + (value >> 13 & 1)) >> 13) as u16
	}
	if exponent >= 31 { return sign | 0x7c00 | u16::from(mantissa != 0) }
	let rounded = mantissa + 0xfff + (mantissa >> 13 & 1);
	if rounded & 0x800000 != 0 { return sign | ((exponent + 1).min(31) as u16) << 10 }
	sign | (exponent as u16) << 10 | (rounded >> 13) as u16
}
#[rustfmt::skip]
fn unfp16(value: u16) -> f32 {
	let sign = (u32::from(value) & 0x8000) << 16;
	let exponent = u32::from(value >> 10 & 31);
	let mantissa = u32::from(value & 1023);
	let bits = if exponent == 0 {
		if mantissa == 0 { sign } else {
			let shift = mantissa.leading_zeros() - 21;
			sign | (113 - shift) << 23 | (mantissa << (shift + 13) & 0x7fffff)
		}
	} else if exponent == 31 { sign | 0x7f800000 | mantissa << 13 }
	else { sign | (exponent + 112) << 23 | mantissa << 13 };
	f32::from_bits(bits)
}
fn put_half(output: &mut Vec<u8>, value: f32) {
	output.extend(fp16(value).to_le_bytes())
}
fn half(input: &[u8]) -> f32 {
	unfp16(u16::from_le_bytes([input[0], input[1]]))
}
fn float(input: &[u8]) -> f32 { f32::from_le_bytes([input[0], input[1], input[2], input[3]]) }
fn qround(value: f32) -> f32 { (((value + 12582912.0).to_bits() as i32 & 0x007fffff) - 0x00400000) as f32 }
fn positive_max(values: &[f32]) -> f32 { values.iter().fold(0.0, |maximum, value| if *value > maximum { *value } else { maximum }) }
#[rustfmt::skip]
fn qkx2(values: &[f32], weights: &[f32], levels: i32, range: (f32, f32, usize), mad: bool, codes: &mut [u8]) -> (f32, f32) {
	let (mut minimum, mut maximum, mut sum_w, mut sum_x) = (values[0], values[0], weights[0], weights[0] * values[0]);
	for index in 1..values.len() { if values[index] < minimum { minimum = values[index] } if values[index] > maximum { maximum = values[index] } sum_w += weights[index]; sum_x += weights[index] * values[index] }
	if minimum > 0.0 { minimum = 0.0 }
	if maximum == minimum { codes.fill(0); return (0.0, -minimum) }
	let mut inverse = levels as f32 / (maximum - minimum); let mut scale = 1.0 / inverse; let mut best_error = 0.0;
	for index in 0..values.len() { codes[index] = qround(inverse * (values[index] - minimum)).max(0.0).min(levels as f32) as u8; let difference = scale * f32::from(codes[index]) + minimum - values[index]; best_error += weights[index] * if mad { difference.abs() } else { difference * difference } }
	let mut trial = vec![0_u8; values.len()];
	for step in 0..=range.2 {
		inverse = (range.0 + range.1 * step as f32 + levels as f32) / (maximum - minimum);
		let (mut sum_l, mut sum_l2, mut sum_xl) = (0.0, 0.0, 0.0);
		for index in 0..values.len() { trial[index] = qround(inverse * (values[index] - minimum)).max(0.0).min(levels as f32) as u8; let code = f32::from(trial[index]); sum_l += weights[index] * code; sum_l2 += weights[index] * code * code; sum_xl += weights[index] * code * values[index] }
		let denominator = sum_w * sum_l2 - sum_l * sum_l;
		if denominator > 0.0 {
			let mut candidate_scale = (sum_w * sum_xl - sum_x * sum_l) / denominator;
			let mut candidate_minimum = (sum_l2 * sum_x - sum_l * sum_xl) / denominator;
			if candidate_minimum > 0.0 { candidate_minimum = 0.0; candidate_scale = sum_xl / sum_l2 }
			let mut error = 0.0; for index in 0..values.len() { let difference = candidate_scale * f32::from(trial[index]) + candidate_minimum - values[index]; error += weights[index] * if mad { difference.abs() } else { difference * difference } }
			if error < best_error { codes.copy_from_slice(&trial); best_error = error; scale = candidate_scale; minimum = candidate_minimum }
		}
	}
	(scale, -minimum)
}
#[rustfmt::skip]
fn q3(values: &[f32], codes: &mut [i8]) -> f32 {
	let (mut maximum, mut absolute) = (0.0_f32, 0.0_f32);
	for value in values { let candidate = value.abs(); if candidate > absolute { absolute = candidate; maximum = *value } }
	if absolute < 1.0e-15 { codes.fill(0); return 0.0 }
	let inverse = -4.0 / maximum;
	let (mut sum_lx, mut sum_l2) = (0.0, 0.0);
	for index in 0..values.len() { let code = qround(inverse * values[index]).max(-4.0).min(3.0); codes[index] = code as i8; let weight = values[index] * values[index]; sum_lx += weight * values[index] * code; sum_l2 += weight * code * code }
	for _ in 0..5 {
		let mut changed = 0;
		for index in 0..values.len() {
			let value = values[index]; let code = f32::from(codes[index]); let weight = value * value; let mut reduced_lx = sum_lx - weight * value * code;
			if reduced_lx > 0.0 { let mut reduced_l2 = sum_l2 - weight * code * code; let candidate = qround(value * reduced_l2 / reduced_lx).max(-4.0).min(3.0); if candidate != code { reduced_lx += weight * value * candidate; reduced_l2 += weight * candidate * candidate; if reduced_l2 > 0.0 && reduced_lx * reduced_lx * sum_l2 > sum_lx * sum_lx * reduced_l2 { codes[index] = candidate as i8; sum_lx = reduced_lx; sum_l2 = reduced_l2; changed += 1 } } }
		}
		if changed == 0 { break }
	}
	for code in codes { *code += 4 }
	if sum_l2 > 0.0 { sum_lx / sum_l2 } else { 0.0 }
}
#[rustfmt::skip]
fn qx(values: &[f32], levels: i32, codes: &mut [i8]) -> f32 {
	let (mut maximum, mut absolute) = (0.0_f32, 0.0_f32);
	for value in values { let candidate = value.abs(); if candidate > absolute { absolute = candidate; maximum = *value } }
	if absolute < 1.0e-15 { codes.fill(0); return 0.0 }
	let mut inverse = -(levels as f32) / maximum;
	let (mut sum_lx, mut sum_l2) = (0.0, 0.0);
	for index in 0..values.len() { let signed = qround(inverse * values[index]).max(-(levels as f32)).min((levels - 1) as f32); codes[index] = signed as i8 + levels as i8; let weight = values[index] * values[index]; sum_lx += weight * values[index] * signed; sum_l2 += weight * signed * signed }
	let mut scale = if sum_l2 == 0.0 { 0.0 } else { sum_lx / sum_l2 };
	let mut best = scale * sum_lx;
	for step in -9..=9 {
		if step == 0 { continue }
		inverse = -(levels as f32 + 0.1 * step as f32) / maximum;
		(sum_lx, sum_l2) = (0.0, 0.0);
		for value in values { let code = qround(inverse * value).max(-(levels as f32)).min((levels - 1) as f32); let weight = value * value; sum_lx += weight * value * code; sum_l2 += weight * code * code }
		if sum_l2 > 0.0 && sum_lx * sum_lx > best * sum_l2 {
			for (value, code) in values.iter().zip(codes.iter_mut()) { *code = qround(inverse * value).max(-(levels as f32)).min((levels - 1) as f32) as i8 + levels as i8 }
			scale = sum_lx / sum_l2; best = scale * sum_lx
		}
	}
	scale
}
fn k_scale(metadata: &[u8], block: usize) -> (u8, u8) { if block < 4 { (metadata[block] & 63, metadata[block + 4] & 63) } else { ((metadata[block + 4] & 15) | (metadata[block - 4] >> 6) << 4, (metadata[block + 4] >> 4) | (metadata[block] >> 6) << 4) } }
const IQ4: [i8; 16] = [-127, -104, -83, -65, -49, -35, -22, -10, 1, 13, 25, 38, 53, 69, 89, 113];
const IQ3_XXS: [u16; 256] = [
	0,2,4,9,11,15,16,18,25,34,59,61,65,67,72,74,81,85,88,90,97,108,120,128,130,132,137,144,146,153,155,159,
	169,175,189,193,199,200,202,213,248,267,287,292,303,315,317,321,327,346,362,413,436,456,460,462,483,497,513,515,520,522,529,531,
	536,538,540,551,552,576,578,585,592,594,641,643,648,650,657,664,698,704,706,720,729,742,758,769,773,808,848,852,870,889,901,978,
	992,1024,1026,1033,1035,1040,1042,1046,1049,1058,1089,1091,1093,1096,1098,1105,1112,1139,1143,1144,1152,1154,1161,1167,1168,1170,1183,1184,1197,1217,1224,1228,
	1272,1276,1309,1323,1347,1367,1377,1404,1473,1475,1486,1509,1537,1544,1546,1553,1555,1576,1589,1594,1600,1602,1616,1625,1636,1638,1665,1667,1672,1685,1706,1722,
	1737,1755,1816,1831,1850,1856,1862,1874,1901,1932,1950,1971,2011,2032,2052,2063,2077,2079,2091,2095,2172,2192,2207,2208,2224,2230,2247,2277,2308,2345,2356,2389,
	2403,2424,2501,2504,2506,2520,2570,2593,2616,2624,2630,2646,2669,2700,2714,2746,2754,2795,2824,2835,2839,2874,2882,2905,2984,3028,3042,3092,3108,3110,3124,3153,
	3185,3215,3252,3288,3294,3364,3397,3434,3483,3523,3537,3587,3589,3591,3592,3610,3626,3670,3680,3722,3749,3754,3776,3789,3803,3824,3857,3873,3904,3906,3924,3992];
const IQ3_S: [u16; 512] = [
	0,1,2,5,7,8,9,10,12,14,16,17,21,27,32,34,37,39,41,43,48,50,57,60,63,64,65,66,68,72,73,77,80,83,87,89,93,100,113,117,122,128,129,133,135,136,139,142,145,149,152,156,162,165,167,169,171,184,187,195,201,205,208,210,217,219,222,228,232,234,247,249,253,256,267,271,273,276,282,288,291,297,312,322,324,336,338,342,347,353,357,359,374,379,390,393,395,409,426,441,448,450,452,464,466,470,475,488,492,512,513,514,516,520,521,523,525,527,528,530,537,540,542,556,558,561,570,576,
	577,579,582,584,588,593,600,603,609,616,618,632,638,640,650,653,655,656,660,666,672,675,685,688,698,705,708,711,712,715,721,727,728,732,737,754,760,771,773,778,780,793,795,802,806,808,812,833,840,843,849,856,858,873,912,916,919,932,934,961,963,968,970,977,989,993,1010,1016,1024,1025,1027,1029,1031,1032,1034,1036,1038,1041,1043,1047,1048,1050,1057,1059,1061,1064,1066,1079,1080,1083,1085,1088,1090,1096,1099,1103,1106,1109,1113,1116,1122,1129,1153,1156,1159,1169,1171,1176,1183,1185,1195,1199,1209,1212,1216,1218,1221,1225,1234,1236,1241,1243,1250,1256,1270,1281,1287,1296,
	1299,1306,1309,1313,1338,1341,1348,1353,1362,1375,1376,1387,1400,1408,1410,1415,1425,1453,1457,1477,1481,1494,1496,1507,1512,1538,1545,1547,1549,1551,1554,1561,1563,1565,1570,1572,1575,1577,1587,1593,1601,1603,1605,1612,1617,1619,1632,1648,1658,1662,1664,1674,1680,1690,1692,1704,1729,1736,1740,1745,1747,1751,1752,1761,1763,1767,1773,1787,1795,1801,1806,1810,1817,1834,1840,1844,1857,1864,1866,1877,1882,1892,1902,1915,1934,1953,1985,1987,2000,2002,2013,2048,2052,2058,2064,2068,2071,2074,2081,2088,2104,2114,2119,2121,2123,2130,2136,2141,2147,2153,2157,2177,2179,2184,2189,2193,2203,2208,2223,2226,2232,2244,2249,2251,2256,2258,2265,2269,
	2304,2306,2324,2335,2336,2361,2373,2375,2385,2418,2443,2460,2480,2504,2509,2520,2531,2537,2562,2568,2572,2578,2592,2596,2599,2602,2614,2620,2625,2627,2629,2634,2641,2650,2682,2688,2697,2707,2712,2718,2731,2754,2759,2760,2775,2788,2793,2805,2811,2817,2820,2832,2842,2854,2890,2902,2921,2923,2978,3010,3012,3026,3081,3083,3085,3097,3099,3120,3136,3152,3159,3188,3210,3228,3234,3245,3250,3256,3264,3276,3281,3296,3349,3363,3378,3392,3395,3420,3440,3461,3488,3529,3531,3584,3588,3591,3600,3602,3614,3616,3628,3634,3650,3657,3668,3683,3685,3713,3716,3720,3726,3729,3736,3753,3778,3802,3805,3819,3841,3845,3851,3856,3880,3922,3938,3970,3993,4032];
const IQ2_XXS:[u16;256]=[0,2,5,8,10,17,20,32,34,40,42,65,68,80,88,97,100,128,130,138,162,257,260,272,277,320,388,408,512,514,546,642,1025,1028,1040,1057,1060,1088,1090,1096,1120,1153,1156,1168,1188,1280,1282,1288,1312,1350,1385,1408,1425,1545,1552,1600,1668,1700,2048,2053,2056,2068,2088,2113,2116,2128,2130,2184,2308,2368,2562,2580,4097,4100,4112,4129,4160,4192,4228,4240,4245,4352,4360,4384,4432,4442,4480,4644,4677,5120,5128,5152,5157,5193,5248,5400,5474,5632,5654,6145,6148,6160,6208,6273,6400,6405,6560,6737,8192,8194,8202,8260,8289,8320,8322,8489,8520,8704,8706,9217,9220,9232,9280,9302,9472,9537,9572,9872,10248,10272,10388,10820,16385,16388,16400,16408,16417,16420,16448,16456,16470,16480,16513,16516,16528,16640,16672,16737,16768,16773,16897,16912,16968,16982,17000,17408,17416,17440,17536,17561,17682,17700,17920,18433,18436,18448,18496,18501,18688,18776,18785,18818,19013,19088,20480,20488,20497,20505,20512,20608,20616,20740,20802,20900,21137,21648,21650,21770,22017,22100,22528,22545,22553,22628,22848,23048,24580,24592,24640,24680,24832,24917,25112,25184,25600,25605,25872,25874,25988,26690,32768,32770,32778,32833,32898,33028,33048,33088,33297,33793,33796,33808,33813,33856,33888,34048,34118,34196,34313,34368,34400,34818,35076,35345,36868,36880,36900,36928,37025,37142,37248,37445,37888,37922,37956,38225,39041,39200,40962,41040,41093,41225,41472,42008,43088,43268];
const IQ2_XS:[u16;512]=[
	0,2,5,8,10,17,20,22,25,32,34,37,40,65,68,70,73,80,82,85,88,97,100,128,130,133,136,145,148,153,160,257,260,262,265,272,274,277,280,282,289,292,320,322,325,328,337,340,352,360,385,388,400,512,514,517,520,529,532,544,577,580,592,597,640,650,1025,1028,1030,1033,1040,1042,1045,1048,1057,1060,1088,1090,1093,1096,1105,1108,1110,1120,1153,1156,1168,1280,1282,1285,1288,1297,1300,1312,1345,1348,1360,1377,1408,1537,1540,1552,1574,1600,1602,1668,2048,2050,2053,2056,2058,2065,2068,2080,2085,2113,2116,2128,2136,2176,2208,2218,2305,2308,2320,2368,2433,2441,2560,2592,2600,2710,2720,4097,4100,4102,4105,4112,4114,4117,4120,4129,4132,4160,4162,4165,4168,4177,4180,4192,4202,4225,4228,4240,4352,4354,4357,4360,4369,4372,4384,4417,4420,4432,4480,4500,4502,4609,4612,4614,4624,4672,4704,5120,5122,5125,5128,5137,5140,5152,5185,5188,5193,5200,5220,5248,5377,5380,5392,5440,5632,5652,5705,6145,6148,6160,6162,6208,6228,6278,6400,6405,6502,6737,6825,8192,8194,8197,8200,8202,8209,8212,8224,8257,8260,8272,8320,8352,8449,8452,8464,8512,8520,8549,8704,8738,8832,8872,9217,9220,9232,9257,9280,9472,9537,9554,9625,9729,9754,9894,10240,10248,10250,10272,10325,10376,10402,10600,10640,10760,10784,10882,10888,10890,16385,16388,
	16390,16393,16400,16402,16405,16408,16417,16420,16448,16450,16453,16456,16458,16465,16468,16480,16485,16513,16516,16528,16640,16642,16645,16648,16657,16660,16672,16705,16708,16720,16768,16773,16802,16897,16900,16912,16914,16937,16960,17408,17410,17413,17416,17425,17428,17433,17440,17473,17476,17488,17536,17556,17665,17668,17680,17700,17728,17818,17920,17930,17988,18000,18433,18436,18448,18496,18501,18516,18530,18688,18705,18756,18768,18793,18948,20480,20482,20485,20488,20497,20500,20512,20520,20545,20548,20560,20608,20737,20740,20752,20757,20800,20802,20992,21060,21162,21505,21508,21520,21537,21568,21600,21633,21665,21760,21768,21888,21896,22049,22120,22177,22528,22548,22593,22608,22681,22810,22848,22850,23173,24577,24580,24592,24640,24660,24674,24710,24745,24832,25124,25162,25234,25600,25622,25872,25920,25925,26020,26625,26730,26917,27142,27220,27234,32768,32770,32773,32776,32785,32788,32800,32810,32833,32836,32848,32896,32898,32936,32938,33025,33028,33030,33040,33088,33105,33113,33280,33312,33408,33410,33440,33448,33793,33796,33808,33810,33813,33856,33888,33929,34048,34116,34213,34328,34410,34816,34824,34853,34906,34944,34946,34984,35078,35362,35456,35464,35478,35496,36865,36868,36880,36928,36950,36996,37120,37154,37220,37462,37513,37888,37893,37956,37968,37976,38185,38288,38290,38465,38993,39078,39241,39445,39520,40960,40962,40968,40970,40992,41002,41120,41297,41305,41382,41472,41474,41480,41514,41600,41632,42048,42133,42597,42648,43018,43040,43042,43048,43168,43176,43268,43396,43398,43560,43562,43665,43690];
const IQ1:[u16;2048]=[
	0,2,5,8,10,17,21,32,34,40,42,69,81,84,86,101,128,130,136,138,149,160,162,168,170,260,261,273,276,278,281,282,293,321,326,329,338,341,346,353,356,358,360,389,401,404,406,421,512,514,520,522,533,544,546,552,554,581,593,601,612,617,640,642,648,650,657,661,665,672,674,680,682,1041,1044,1046,1061,1089,1097,1109,1114,1124,1125,1169,1177,1189,1281,1284,1285,1286,1301,1304,1306,1321,1344,1349,1354,1360,1361,1364,1365,1366,1369,1376,1378,1381,1384,1386,1409,1425,1429,1432,1434,1441,1444,1445,1446,1449,1556,1561,1601,1604,1616,1618,1621,1624,1632,1633,1638,1641,1669,1681,1684,1689,2048,2050,2056,2058,2069,2080,2082,2088,2090,2117,2129,2134,2149,2176,2178,2184,2186,2197,2208,2210,2216,2218,2309,2321,2324,2329,2340,2341,2369,2384,2385,2389,2401,2404,2409,2449,2452,2454,2457,2469,2560,2562,2568,2570,2581,2592,2594,2600,2602,2629,2641,2649,2657,2661,2688,2690,2693,2696,2698,2709,2720,2722,2728,2730,4112,4113,4116,4121,4132,4133,4161,4164,4176,4181,4184,4193,4196,4197,4201,4241,4244,4246,4257,4261,4353,4356,4358,4361,4368,4370,4373,4376,4385,4388,4393,4421,4426,4432,4433,4434,4436,4437,4438,4441,4448,4453,4484,4498,4501,4513,4516,4625,4628,4630,4645,4672,4678,4681,4690,4693,4696,4698,
	4708,4710,4741,4753,4756,4758,4773,5121,5126,5129,5140,5141,5144,5145,5153,5158,5185,5189,5190,5192,5194,5201,5204,5205,5206,5209,5218,5221,5224,5252,5257,5264,5268,5269,5272,5273,5274,5281,5284,5285,5289,5378,5381,5386,5393,5396,5397,5398,5401,5408,5410,5413,5416,5418,5441,5444,5445,5446,5457,5458,5460,5461,5462,5465,5466,5473,5476,5477,5478,5481,5504,5506,5508,5509,5512,5514,5520,5521,5524,5525,5526,5529,5530,5536,5538,5541,5633,5636,5637,5638,5653,5654,5656,5658,5665,5670,5696,5698,5700,5701,5704,5706,5713,5717,5718,5720,5721,5729,5732,5733,5736,5737,5738,5766,5770,5778,5781,5796,5801,6161,6166,6181,6209,6212,6214,6217,6224,6229,6232,6234,6240,6241,6244,6246,6249,6277,6289,6292,6309,6416,6418,6421,6426,6433,6437,6466,6468,6469,6472,6481,6484,6485,6486,6489,6490,6496,6501,6506,6537,6545,6546,6549,6552,6561,6566,6569,6665,6678,6692,6694,6724,6726,6729,6736,6738,6741,6744,6753,6758,6761,6789,6801,6806,6810,8192,8194,8200,8202,8213,8224,8226,8229,8232,8234,8261,8273,8281,8289,8293,8320,8322,8328,8330,8341,8352,8354,8357,8360,8362,8453,8465,8468,8473,8485,8514,8516,8521,8533,8536,8538,8545,8548,8549,8550,8581,8592,8598,8601,8613,8705,8712,8714,8721,8725,8736,8738,8744,8746,8773,8785,8790,8793,8805,8833,8840,8842,8849,8853,8864,8866,8872,8874,9221,9236,9238,9241,
	9253,9284,9285,9286,9289,9298,9301,9304,9306,9318,9349,9361,9364,9369,9377,9381,9481,9493,9505,9513,9536,9541,9544,9553,9556,9557,9561,9570,9573,9576,9609,9616,9620,9621,9624,9626,9633,9636,9638,9641,9733,9744,9746,9753,9765,9793,9801,9813,9824,9825,9833,9860,9862,9872,9882,10240,10242,10248,10250,10261,10272,10274,10280,10282,10309,10321,10324,10341,10368,10370,10376,10378,10400,10402,10408,10410,10505,10513,10516,10521,10533,10566,10569,10578,10581,10593,10596,10598,10601,10629,10640,10646,10649,10660,10661,10752,10754,10760,10762,10784,10786,10792,10794,10821,10833,10838,10841,10853,10880,10882,10888,10890,10901,10912,10914,10920,10922,16389,16401,16406,16421,16457,16466,16469,16472,16474,16481,16484,16486,16532,16537,16545,16550,16640,16641,16644,16646,16649,16658,16661,16662,16664,16666,16673,16678,16681,16709,16712,16714,16721,16724,16725,16726,16729,16730,16741,16744,16746,16769,16772,16774,16784,16786,16789,16800,16801,16802,16901,16913,16916,16918,16933,16961,16978,16981,16986,16996,17001,17033,17044,17061,17409,17429,17433,17449,17477,17480,17482,17489,17492,17493,17494,17505,17506,17509,17512,17514,17537,17542,17545,17552,17554,17557,17568,17569,17577,17665,17666,17669,17674,17681,17684,17685,17686,17689,17696,17701,17706,17729,17732,17733,17734,17737,17744,17745,17748,17749,17750,17752,17753,17761,17764,17765,17766,17769,17794,17796,17797,17800,17809,17812,17813,17814,17817,17818,17829,17832,17834,17921,17925,17929,17940,17941,17944,17946,17953,
	17956,17961,17984,17986,17989,17992,18000,18001,18002,18005,18006,18009,18018,18021,18024,18049,18053,18058,18068,18069,18081,18084,18086,18437,18449,18453,18458,18469,18498,18505,18512,18517,18520,18529,18532,18534,18537,18565,18577,18580,18582,18585,18597,18689,18693,18694,18698,18704,18708,18709,18712,18721,18724,18726,18752,18757,18762,18769,18770,18772,18773,18774,18777,18784,18786,18789,18790,18794,18822,18825,18834,18837,18838,18840,18849,18852,18854,18857,18966,19012,19014,19017,19029,19032,19034,19044,19049,19092,19109,20481,20484,20485,20486,20489,20498,20501,20506,20513,20516,20521,20544,20549,20552,20561,20564,20565,20566,20569,20581,20584,20614,20617,20629,20632,20640,20641,20646,20649,20741,20744,20745,20746,20753,20756,20757,20758,20760,20761,20768,20773,20774,20776,20778,20801,20804,20805,20806,20809,20816,20817,20818,20820,20821,20822,20824,20825,20826,20833,20836,20837,20838,20841,20866,20869,20881,20884,20885,20886,20889,20896,20901,20906,20993,20998,21010,21013,21018,21025,21028,21058,21061,21066,21073,21076,21077,21078,21081,21090,21093,21125,21136,21138,21141,21145,21146,21156,21508,21509,21521,21524,21525,21526,21528,21529,21537,21541,21544,21546,21569,21572,21573,21574,21577,21578,21584,21585,21588,21589,21590,21592,21593,21594,21601,21602,21604,21605,21606,21609,21632,21640,21642,21649,21652,21653,21654,21657,21665,21668,21669,21674,21761,21762,21764,21765,21766,21769,21776,21777,21778,21780,21781,21782,21785,21786,21793,21796,21797,21798,21801,21824,21825,21826,21828,21829,21830,21832,
	21833,21840,21841,21842,21844,21845,21846,21848,21849,21850,21856,21857,21860,21861,21862,21864,21865,21866,21889,21892,21893,21897,21898,21904,21905,21908,21909,21910,21912,21913,21921,21924,21925,21926,21929,22016,22017,22018,22020,22022,22024,22025,22033,22036,22037,22040,22041,22048,22049,22050,22052,22053,22054,22056,22057,22081,22085,22086,22088,22089,22090,22096,22097,22098,22100,22101,22102,22104,22105,22106,22113,22116,22117,22121,22146,22149,22150,22152,22153,22154,22161,22165,22170,22178,22181,22182,22184,22185,22532,22533,22534,22537,22544,22549,22552,22561,22570,22597,22600,22602,22609,22612,22613,22614,22616,22617,22624,22626,22628,22629,22658,22665,22672,22674,22677,22680,22689,22697,22785,22786,22789,22794,22801,22804,22805,22806,22809,22821,22849,22852,22853,22854,22857,22864,22865,22866,22868,22869,22870,22872,22873,22874,22881,22884,22885,22886,22889,22913,22917,22921,22929,22932,22933,22934,22936,22937,22949,23044,23048,23061,23066,23072,23077,23078,23081,23109,23112,23113,23121,23125,23126,23128,23129,23138,23141,23144,23146,23169,23178,23186,23189,23190,23192,23194,23201,24581,24596,24598,24601,24613,24644,24656,24661,24662,24664,24666,24673,24676,24678,24681,24705,24726,24741,24833,24836,24838,24841,24850,24853,24865,24866,24870,24873,24901,24905,24913,24917,24918,24921,24933,24934,24938,24964,24970,24978,24981,24993,24998,25001,25105,25110,25113,25152,25153,25158,25173,25174,25176,25184,25221,25233,25238,25253,25617,25618,25621,25622,25626,25633,25638,25641,25664,25666,25669,25672,25674,
	25681,25684,25685,25686,25689,25690,25696,25698,25701,25732,25733,25737,25744,25746,25748,25749,25750,25752,25754,25761,25764,25769,25861,25864,25866,25873,25877,25878,25881,25924,25925,25926,25929,25936,25937,25940,25941,25942,25945,25953,25956,25957,25958,25961,25990,25993,25994,26001,26005,26006,26009,26010,26018,26021,26022,26024,26114,26121,26133,26144,26150,26152,26153,26176,26181,26184,26186,26193,26196,26197,26198,26200,26202,26208,26213,26216,26240,26242,26245,26250,26260,26262,26264,26265,26272,26276,26278,26282,26646,26649,26661,26689,26706,26709,26714,26721,26729,26757,26769,26776,26790,26881,26884,26896,26901,26913,26916,26918,26921,26944,26945,26949,26950,26952,26961,26964,26965,26966,26969,26976,26981,26986,27010,27012,27018,27029,27041,27044,27045,27049,27153,27158,27160,27201,27204,27209,27216,27221,27224,27226,27236,27237,27241,27270,27284,27288,27290,27302,32768,32770,32776,32778,32800,32802,32808,32810,32837,32848,32849,32852,32854,32857,32869,32896,32898,32904,32906,32917,32928,32930,32936,32938,33029,33041,33044,33046,33049,33061,33089,33092,33097,33104,33106,33109,33110,33112,33113,33124,33126,33129,33157,33161,33172,33174,33177,33189,33280,33282,33288,33290,33301,33312,33314,33320,33322,33361,33364,33369,33381,33408,33410,33416,33418,33429,33440,33442,33448,33450,33812,33817,33857,33860,33873,33877,33882,33889,33892,33897,33940,33945,34049,34057,34066,34069,34074,34086,34089,34112,34113,34117,34120,34129,34132,34133,34134,34137,34138,34149,34150,34152,34154,34177,34180,34182,34185,34192,
	34194,34197,34200,34214,34321,34326,34329,34341,34369,34372,34377,34378,34384,34389,34393,34394,34401,34406,34410,34437,34449,34458,34468,34816,34818,34824,34826,34837,34848,34850,34856,34858,34881,34885,34897,34900,34905,34917,34921,34944,34946,34952,34954,34965,34976,34978,34984,34986,35077,35078,35089,35092,35094,35109,35137,35140,35142,35145,35152,35154,35157,35162,35169,35172,35205,35222,35225,35237,35328,35330,35336,35338,35349,35360,35362,35368,35370,35397,35409,35412,35414,35456,35458,35464,35466,35477,35488,35490,35496,35498,36869,36881,36886,36888,36889,36901,36929,36934,36937,36949,36952,36954,36969,36970,36997,37009,37012,37014,37017,37029,37121,37124,37126,37129,37136,37141,37144,37146,37153,37156,37158,37161,37184,37189,37200,37201,37204,37205,37206,37209,37218,37221,37252,37254,37266,37269,37272,37281,37284,37286,37289,37381,37393,37396,37401,37413,37444,37446,37449,37456,37458,37461,37464,37478,37481,37509,37524,37526,37545,37889,37892,37894,37904,37909,37912,37926,37952,37962,37969,37972,37973,37974,37976,37977,37984,37985,37986,37989,38020,38022,38034,38036,38037,38040,38049,38057,38144,38149,38152,38154,38160,38161,38164,38165,38166,38169,38177,38181,38185,38186,38209,38212,38213,38214,38217,38224,38225,38226,38228,38229,38230,38232,38233,38234,38241,38244,38245,38246,38249,38273,38277,38280,38289,38290,38292,38293,38294,38297,38298,38304,38306,38309,38312,38314,38401,38404,38416,38421,38425,38432,38438,38441,38469,38472,38473,38481,38482,38485,38486,38489,38501,38504,38530,38532,38537,38538,
	38546,38548,38549,38564,38566,38569,38917,38934,38937,38949,38977,38982,38992,38994,38997,38998,39002,39012,39013,39045,39057,39062,39065,39077,39172,39174,39177,39184,39186,39189,39192,39194,39200,39201,39204,39206,39232,39234,39237,39240,39242,39249,39252,39253,39254,39257,39266,39269,39270,39274,39297,39300,39312,39314,39317,39322,39329,39334,39429,39445,39461,39492,39494,39497,39504,39509,39512,39521,39557,39569,39572,39573,39574,40960,40962,40968,40970,40981,40992,40994,41000,41002,41029,41041,41044,41046,41049,41088,41090,41096,41098,41109,41120,41122,41128,41130,41221,41225,41233,41236,41238,41241,41242,41286,41289,41297,41301,41304,41306,41313,41316,41349,41360,41362,41366,41369,41474,41480,41482,41488,41497,41506,41512,41514,41541,41553,41558,41561,41573,41600,41602,41608,41610,41621,41632,41634,41640,41642,42009,42021,42049,42052,42064,42068,42069,42072,42074,42081,42085,42086,42088,42089,42117,42246,42249,42256,42258,42261,42264,42278,42281,42306,42309,42321,42324,42325,42326,42329,42341,42346,42369,42372,42373,42374,42377,42386,42389,42392,42501,42513,42518,42522,42529,42533,42564,42566,42570,42578,42581,42582,42584,42592,42594,42630,42640,42645,42646,42649,42657,42660,42662,43008,43010,43016,43018,43040,43042,43048,43050,43089,43092,43094,43097,43136,43138,43144,43146,43157,43168,43170,43176,43178,43269,43284,43289,43297,43301,43329,43344,43349,43354,43361,43366,43369,43408,43414,43520,43522,43528,43530,43552,43554,43560,43562,43601,43604,43606,43648,43650,43656,43658,43669,43680,43682,43688,43690];
const IQ2_S: [u16; 1024] = [
	0,2,5,8,10,17,20,22,25,32,34,37,40,65,68,70,73,80,82,85,88,97,100,102,105,128,130,133,136,145,148,160,165,170,257,260,262,265,272,274,277,280,289,292,320,322,325,328,337,340,342,345,352,357,360,385,388,400,402,405,417,420,512,514,517,520,529,532,544,554,577,580,582,585,592,597,640,645,650,660,674,1025,1028,1030,1033,1040,1042,1045,1048,1057,1060,1062,1065,1088,1090,1093,1096,1098,1105,1108,1110,1113,1120,1122,1125,1153,1156,1158,1161,1168,1173,1176,1185,1188,1280,1282,1285,1288,1290,1297,1300,1302,1305,1312,1317,1320,1345,1348,1350,1353,1360,1362,1365,1368,1377,1380,1408,1410,1413,1416,1425,1428,1440,1537,1540,1542,1545,1552,1557,1600,1605,1608,1617,1620,1632,1665,1668,1680,2048,2050,2053,2056,2065,2068,2070,2073,2080,2085,2090,2113,2116,2118,2121,2128,2130,2133,2136,2145,2148,2176,2181,2196,2218,2305,2308,2320,2322,2325,2328,2337,2368,2373,2376,2385,2388,2400,2433,2448,2560,2577,2580,2594,2600,2602,2640,2713,4097,4100,4102,4105,4112,4114,4117,4120,4129,4132,4134,4160,4162,4165,4168,4177,4180,4182,4185,4192,4194,4197,4200,4225,4228,4230,4240,4245,4248,4257,4260,4352,4354,4357,4360,4362,4369,4372,4374,4377,4384,4386,4389,4392,4417,4420,4422,4425,4432,4434,
	4437,4440,4449,4452,4480,4482,4485,4488,4497,4500,4609,4612,4617,4624,4629,4641,4644,4672,4677,4689,4692,4737,4740,4752,5120,5122,5125,5128,5137,5140,5142,5145,5152,5157,5160,5185,5188,5190,5193,5200,5202,5205,5208,5217,5220,5248,5250,5253,5256,5265,5268,5280,5377,5380,5382,5385,5392,5394,5397,5400,5409,5412,5440,5442,5445,5448,5457,5460,5472,5505,5508,5520,5632,5637,5640,5649,5652,5664,5697,5700,5712,5760,5802,6145,6148,6150,6153,6160,6165,6168,6177,6208,6210,6213,6216,6225,6228,6240,6273,6276,6400,6402,6405,6408,6417,6420,6432,6465,6468,6480,6505,6562,6660,6672,6720,6742,8192,8194,8197,8200,8209,8212,8214,8217,8224,8229,8234,8257,8260,8272,8274,8277,8292,8320,8330,8340,8362,8449,8452,8464,8466,8469,8481,8512,8514,8517,8529,8532,8544,8577,8580,8592,8704,8714,8738,8744,8746,8772,8784,8840,8842,8872,9217,9220,9222,9225,9232,9237,9240,9249,9252,9280,9282,9285,9288,9297,9300,9312,9345,9348,9360,9472,9477,9480,9489,9492,9504,9537,9540,9552,9574,9600,9729,9732,9744,9792,9817,10240,10245,10257,10260,10305,10308,10320,10378,10410,10497,10500,10512,10645,10762,10786,10852,10888,10890,16385,16388,16390,16393,16400,16402,16405,16408,16410,16417,16420,16422,16448,16450,16453,16456,16458,16465,16468,16470,16473,16480,16482,16485,16513,16516,16528,16533,16536,16545,16548,16640,16642,16645,16648,16657,16660,16662,16665,16672,16674,
	16677,16705,16708,16710,16713,16720,16722,16725,16728,16737,16740,16768,16770,16773,16776,16785,16788,16800,16897,16900,16912,16914,16917,16920,16932,16960,16965,16968,16977,16980,16992,17025,17028,17408,17410,17413,17416,17418,17425,17428,17430,17433,17440,17442,17445,17448,17473,17476,17478,17481,17488,17490,17493,17496,17505,17508,17536,17538,17541,17544,17553,17556,17568,17665,17668,17670,17673,17680,17682,17685,17688,17697,17700,17728,17730,17733,17736,17745,17748,17760,17770,17793,17796,17808,17920,17922,17925,17928,17937,17940,17952,17985,17988,18000,18048,18085,18433,18436,18441,18448,18450,18453,18456,18465,18468,18496,18498,18501,18504,18513,18516,18528,18564,18576,18688,18690,18693,18696,18705,18708,18720,18753,18756,18768,18816,18838,18945,18948,18960,19008,20480,20482,20485,20488,20497,20500,20502,20505,20512,20514,20517,20520,20545,20548,20550,20553,20560,20562,20565,20568,20577,20580,20608,20610,20613,20616,20625,20628,20737,20740,20742,20745,20752,20754,20757,20760,20769,20772,20800,20802,20805,20808,20817,20820,20832,20865,20868,20880,20992,20997,21000,21009,21012,21024,21057,21060,21072,21097,21120,21505,21508,21510,21513,21520,21522,21525,21528,21537,21540,21568,21570,21573,21576,21585,21588,21600,21633,21636,21648,21760,21762,21765,21768,21777,21780,21792,21825,21828,21840,21888,22017,22020,22032,22054,22080,22528,22530,22533,22536,22545,22548,22560,22593,22596,22608,22618,22656,22785,22788,22800,22848,23040,23065,23173,23208,24577,24580,24582,24592,24594,24597,24600,24609,24612,24640,24645,
	24648,24657,24660,24672,24708,24720,24832,24834,24837,24840,24849,24852,24864,24897,24900,24912,24960,24985,25092,25104,25152,25174,25249,25600,25605,25608,25617,25620,25632,25665,25668,25680,25728,25857,25860,25872,25920,25930,25960,26002,26112,26260,26625,26628,26640,26725,26776,26880,26922,27202,27297,32768,32770,32773,32776,32785,32788,32793,32800,32805,32833,32836,32848,32850,32853,32856,32865,32896,32901,32913,32916,33025,33028,33033,33040,33042,33045,33048,33057,33060,33088,33090,33093,33096,33105,33108,33153,33156,33168,33193,33280,33285,33290,33297,33300,33345,33348,33360,33793,33796,33798,33801,33808,33810,33813,33816,33825,33856,33858,33861,33864,33873,33876,33888,33921,33924,33936,34048,34050,34053,34056,34065,34068,34080,34113,34116,34128,34176,34186,34305,34308,34320,34345,34368,34816,34821,34833,34836,34881,34884,34896,34978,35073,35076,35136,35173,35362,35416,35418,35458,35490,36865,36868,36873,36880,36882,36885,36888,36900,36928,36930,36933,36936,36945,36948,36960,36993,36996,37008,37120,37125,37137,37140,37185,37188,37200,37210,37377,37380,37392,37440,37542,37888,37890,37893,37896,37905,37908,37920,37953,37956,37968,38016,38038,38145,38148,38160,38208,38296,38305,38400,38470,38500,38913,38916,38928,38950,38976,39081,39168,39241,39250,39568,40960,40965,40970,40980,40994,41002,41025,41028,41040,41122,41130,41280,41317,41474,41482,41506,41512,41514,41602,41608,41610,41640,41985,41988,42000,42048,42121,42148,42240,42265,42577,43018,43048,43170,43348,43398,43528,43530,43552,43554,43560,43656,43690];
fn iq3_grid(grid: &[u16], index: usize, lane: usize) -> i8 { (2 * (grid[index] >> (3 * lane) & 7) + 1) as i8 }
fn iq3_nearest(grid: &[u16], levels: &mut [i8], values: &[f32], weights: &[f32], scale: f32) -> usize {
	let key = levels.iter().enumerate().fold(0_u16, |key, (lane, level)| key | (*level as u16) << (3 * lane));
	if let Some(index) = grid.iter().position(|value| *value == key) { return index }
	let mut candidates = grid.iter().enumerate().map(|(index, point)| ((0..4).map(|lane| { let difference = i32::from((*point >> (3 * lane) & 7) as i8 - levels[lane]); difference * difference }).sum::<i32>(), index)).collect::<Vec<_>>(); candidates.sort_unstable();
	let first = candidates[0].0; let second = candidates.iter().find(|item| item.0 != first).map(|item| item.0).unwrap_or(first);
	let index = candidates.into_iter().take_while(|item| item.0 <= second).map(|item| item.1).min_by(|left, right| { let error = |index| (0..4).map(|lane| { let difference = scale * f32::from(iq3_grid(grid, index, lane)) - values[lane]; weights[lane] * difference * difference }).sum::<f32>(); error(*left).total_cmp(&error(*right)) }).unwrap();
	for lane in 0..4 { levels[lane] = (iq3_grid(grid, index, lane) - 1) / 2 }
	index
}
fn iq2_grid(grid:&[u16],index:usize,lane:usize)->i8{(2*(grid[index]>>(2*lane)&3)+1)as i8}
fn iq2_nearest(grid:&[u16],shells:usize,levels:&mut[i8],values:&[f32],weights:&[f32],scale:f32)->usize{
	let key=levels.iter().enumerate().fold(0_u16,|key,(lane,level)|key|(*level as u16)<<(2*lane));if let Some(index)=grid.iter().position(|value|*value==key){return index}let mut candidates=grid.iter().enumerate().map(|(index,point)|((0..8).map(|lane|{let difference=i32::from((*point>>(2*lane)&3)as i8-levels[lane]);difference*difference}).sum::<i32>(),index)).collect::<Vec<_>>();candidates.sort_unstable();let mut distances=candidates.iter().map(|item|item.0).collect::<Vec<_>>();distances.dedup();let limit=distances.get(shells.saturating_sub(1)).copied().unwrap_or(candidates[0].0);let index=candidates.into_iter().take_while(|item|item.0<=limit).map(|item|item.1).min_by(|left,right|{let error=|index|(0..8).map(|lane|{let difference=scale*f32::from(iq2_grid(grid,index,lane))-values[lane];weights[lane]*difference*difference}).sum::<f32>();error(*left).total_cmp(&error(*right))}).unwrap();for lane in 0..8{levels[lane]=(iq2_grid(grid,index,lane)-1)/2}index
}
fn iq1_level(index:usize,lane:usize)->i8{(IQ1[index]>>(2*lane)&3)as i8}
fn iq1_nearest(levels:&mut[i8],values:&[f32],weights:&[f32],scale:f32,shift:i8)->usize{
	let key=levels.iter().enumerate().fold(0_u16,|key,(lane,level)|key|(*level as u16)<<(2*lane));if let Some(index)=IQ1.iter().position(|value|*value==key){return index}let mut candidates=IQ1.iter().enumerate().map(|(index,point)|((0..8).map(|lane|{let difference=i32::from((*point>>(2*lane)&3)as i8-levels[lane]);difference*difference}).sum::<i32>(),index)).collect::<Vec<_>>();candidates.sort_unstable();let mut distances=candidates.iter().map(|item|item.0).collect::<Vec<_>>();distances.dedup();let limit=distances.get(2).copied().unwrap_or(candidates[0].0);let index=candidates.into_iter().take_while(|item|item.0<=limit).map(|item|item.1).min_by(|left,right|{let error=|index|(0..8).map(|lane|{let quant=f32::from(iq1_level(index,lane))-1.0+0.125*f32::from(shift);let difference=scale*quant-values[lane];weights[lane]*difference*difference}).sum::<f32>();error(*left).total_cmp(&error(*right))}).unwrap();for lane in 0..8{levels[lane]=iq1_level(index,lane)}index
}
fn iq1_shift(medium:bool,pattern:i8,group:usize)->i8{if (!medium&&pattern==0)||(medium&&if group==0{pattern<2}else{pattern%2==0}){1}else{-1}}
#[rustfmt::skip] fn iq1(values:&[f32],importance:&[f32],medium:bool)->Vec<u8>{
	let mut output=Vec::new();for(chunk,values)in values.chunks(256).enumerate(){let value=|index|values.get(index).copied().unwrap_or(0.0);let importance=|index|importance.get(chunk*256+index).copied().unwrap_or(0.0);let sigma2=2.0*(0..256).map(|index|value(index)*value(index)).sum::<f32>()/256.0;let size=if medium{16}else{32};let blocks=256/size;let mut packed=vec![0_u8;if medium{56}else{48}];let mut scales=vec![0.0_f32;blocks];let mut patterns=vec![0_i8;blocks];let mut maximum=0.0_f32;
		for block in 0..blocks{let x=(0..size).map(|offset|value(block*size+offset)).collect::<Vec<_>>();let weights=(0..size).map(|offset|importance(block*size+offset)*(sigma2+x[offset]*x[offset]).sqrt()).collect::<Vec<_>>();let max=x.iter().map(|value|value.abs()).fold(0.0_f32,f32::max);let mut levels=vec![1_i8;size];if max<if medium{1.0e-7}else{1.0e-12}{continue}let mut pairs=x.iter().copied().enumerate().map(|(index,value)|(value,index)).collect::<Vec<_>>();pairs.sort_by(|left,right|left.0.total_cmp(&right.0));let(mut sumx,mut sumw)=(vec![0.0_f32;size+1],vec![0.0_f32;size+1]);for j in 0..size{let index=pairs[j].1;sumx[j+1]=sumx[j]+weights[index]*x[index];sumw[j+1]=sumw[j]+weights[index]}let(mut best,mut scale,mut split,mut pattern)=(f32::NEG_INFINITY,max,(0,0),-1_i8);
			for first in 0..=size{for second in first..=size{for candidate in if medium{&[0_i8,1,2,3][..]}else{&[0_i8,3][..]}{let(mut qx,mut q2)=(0.0_f32,0.0_f32);if medium{for(index,pair)in pairs.iter().enumerate(){let lane=pair.1;let level=if index<first{0.0}else if index<second{1.0}else{2.0};let q=level-1.0+0.125*f32::from(iq1_shift(true,*candidate,lane/8));qx+=weights[lane]*q*x[lane];q2+=weights[lane]*q*q}}else{let shift=iq1_shift(false,*candidate,0);let q=[-1.0+0.125*f32::from(shift),0.125*f32::from(shift),1.0+0.125*f32::from(shift)];qx=(sumx[first]-sumx[0])*q[0]+(sumx[second]-sumx[first])*q[1]+(sumx[size]-sumx[second])*q[2];q2=(sumw[first]-sumw[0])*q[0]*q[0]+(sumw[second]-sumw[first])*q[1]*q[1]+(sumw[size]-sumw[second])*q[2]*q[2]}if q2>0.0&&qx*qx>best*q2{scale=qx/q2;best=scale*qx;split=(first,second);pattern=*candidate}}}}if pattern<0{continue}for(index,pair)in pairs.iter().enumerate(){levels[pair.1]=if index<split.0{0}else if index<split.1{1}else{2}}if scale<0.0{for level in &mut levels{*level=2-*level}scale=-scale;pattern=3-pattern}
			let mut indices=vec![0_usize;size/8];let mut changed=false;for group in 0..size/8{let key=(0..8).fold(0_u16,|key,lane|key|(levels[group*8+lane]as u16)<<(2*lane));changed|=!IQ1.contains(&key);indices[group]=iq1_nearest(&mut levels[group*8..group*8+8],&x[group*8..group*8+8],&weights[group*8..group*8+8],scale,iq1_shift(medium,pattern,group))}if changed{let(mut qx,mut q2)=(0.0,0.0);for lane in 0..size{let quant=f32::from(levels[lane])-1.0+0.125*f32::from(iq1_shift(medium,pattern,lane/8));qx+=weights[lane]*quant*x[lane];q2+=weights[lane]*quant*quant}if qx>0.0&&q2>0.0{scale=qx/q2}}if medium{for group in 0..2{packed[block*2+group]=indices[group]as u8}packed[32+block]=((indices[0]>>8)as u8)|((indices[1]>>8)as u8)<<4|[0,128,8,136][pattern as usize]}else{let mut high=0_u16;for group in 0..4{packed[block*4+group]=indices[group]as u8;high|=((indices[group]>>8)as u16)<<(3*group)}packed[32+2*block..34+2*block].copy_from_slice(&high.to_le_bytes())}scales[block]=scale;patterns[block]=pattern;maximum=maximum.max(scale)}
		if maximum==0.0{if !medium{put_half(&mut output,0.0)}output.extend(packed);continue}let mut scale=maximum/15.0;if medium{let(mut qx,mut q2)=(0.0,0.0);for block in 0..16{let code=qround(0.5*(scales[block]/scale-1.0)).max(0.0).min(7.0)as u16;let word=block/4;let mut stored=u16::from_le_bytes(packed[48+2*word..50+2*word].try_into().unwrap());stored|=code<<(3*(block%4));packed[48+2*word..50+2*word].copy_from_slice(&stored.to_le_bytes());for lane in 0..16{let group=lane/8;let grid=usize::from(packed[2*block+group])|usize::from(packed[32+block]>>(4*group)&7)<<8;let quant=(f32::from(iq1_level(grid,lane%8))-1.0+0.125*f32::from(iq1_shift(true,patterns[block],group)))*f32::from(2*code+1);let x=value(block*16+lane);let weight=importance(block*16+lane)*(sigma2+x*x).sqrt();qx+=weight*quant*x;q2+=weight*quant*quant}}if q2>0.0{scale=qx/q2}
			let bits=fp16(scale*1.1125);for word in 0..4{let mut stored=u16::from_le_bytes(packed[48+2*word..50+2*word].try_into().unwrap());stored|=(bits>>(4*word)&15)<<12;packed[48+2*word..50+2*word].copy_from_slice(&stored.to_le_bytes())}output.extend(packed)}else{for block in 0..8{let code=qround(0.5*(scales[block]/scale-1.0)).max(0.0).min(7.0)as u16|u16::from(patterns[block]!=0)<<3;let mut high=u16::from_le_bytes(packed[32+2*block..34+2*block].try_into().unwrap());high|=code<<12;packed[32+2*block..34+2*block].copy_from_slice(&high.to_le_bytes())}put_half(&mut output,scale*1.125);output.extend(packed)}}output
}
fn qp_scale(values:&[f32],weights:&[f32],nmax:i8)->f32{
	let max=values.iter().copied().fold(0.0_f32,f32::max);if max<1.0e-15{return 0.0}let mut inverse=f32::from(nmax)/max;let mut levels=values.iter().map(|value|qround(inverse*value).min(f32::from(nmax))as i8).collect::<Vec<_>>();let error=|inverse:f32|values.iter().zip(weights).map(|(value,weight)|{let level=qround(inverse*value).min(f32::from(nmax));let difference=value-level/inverse;weight*difference*difference}).sum::<f32>();let mut best=error(inverse);
	for step in -4..=4{if step==0{continue}let trial=(f32::from(nmax)+0.1*step as f32)/max;let trial_error=error(trial);if trial_error<best{best=trial_error;inverse=trial}}let(mut qx,mut q2)=(0.0,0.0);for lane in 0..values.len(){levels[lane]=qround(inverse*values[lane]).min(f32::from(nmax))as i8;qx+=weights[lane]*values[lane]*f32::from(levels[lane]);q2+=weights[lane]*f32::from(levels[lane]*levels[lane])}
	for _ in 0..5{let mut changed=false;for lane in 0..values.len(){let level=f32::from(levels[lane]);let x=qx-weights[lane]*values[lane]*level;let q=q2-weights[lane]*level*level;if x>0.0&&q>0.0{let next=qround(values[lane]*q/x).min(f32::from(nmax))as i8;if next!=levels[lane]{let nx=x+weights[lane]*values[lane]*f32::from(next);let nq=q+weights[lane]*f32::from(next*next);if nx*nx*q2>qx*qx*nq{levels[lane]=next;qx=nx;q2=nq;changed=true}}}}if !changed{break}}if q2>0.0{qx/q2}else{0.0}
}
#[rustfmt::skip] fn iq2_xxs(values:&[f32],importance:&[f32])->Vec<u8>{
	let mut output=Vec::new();for (chunk,values)in values.chunks(256).enumerate(){let value=|index|values.get(index).copied().unwrap_or(0.0);let importance=|index|importance.get(chunk*256+index).copied().unwrap_or(0.0);let sigma2=(0..256).map(|index|value(index)*value(index)).sum::<f32>()/256.0;let mut packed=[0_u8;64];let mut scales=[0.0_f32;8];let mut maximum=0.0_f32;
		for block in 0..8{let x=(0..32).map(|offset|value(block*32+offset)).collect::<Vec<_>>();let weights=(0..32).map(|offset|importance(block*32+offset)*(sigma2+x[offset]*x[offset]).sqrt()).collect::<Vec<_>>();let mut magnitudes=x.iter().map(|value|value.abs()).collect::<Vec<_>>();let mut signs=[0_u8;4];for group in 0..4{let mut flips=0;for lane in 0..8{if x[group*8+lane]<0.0{flips+=1;signs[group]|=1<<lane}}if flips%2!=0{let lane=(0..8).min_by(|a,b|(weights[group*8+*a]*x[group*8+*a]*x[group*8+*a]).total_cmp(&(weights[group*8+*b]*x[group*8+*b]*x[group*8+*b]))).unwrap();magnitudes[group*8+lane]=-magnitudes[group*8+lane];signs[group]^=1<<lane}signs[group]&=127}let max=magnitudes.iter().copied().fold(0.0_f32,f32::max);if max<1.0e-15{continue}
			let seed=qp_scale(&magnitudes,&weights,4);let effective=seed*3.0;if effective<=0.0{continue}let mut best=0.0_f32;let mut scale=seed;let mut levels=[0_i8;32];for step in -6..=6{let inverse=(5.0+0.1*step as f32)/effective;let trial_scale=inverse.recip();let mut trial=[0_i8;32];for group in 0..4{for lane in 0..8{trial[group*8+lane]=qround(0.5*(inverse*magnitudes[group*8+lane]-1.0)).max(0.0).min(2.0)as i8}iq2_nearest(&IQ2_XXS,2,&mut trial[group*8..group*8+8],&magnitudes[group*8..group*8+8],&weights[group*8..group*8+8].iter().map(|value|value.sqrt()).collect::<Vec<_>>(),trial_scale);}let(mut qx,mut q2)=(0.0,0.0);for lane in 0..32{let quant=f32::from(2*trial[lane]+1);qx+=weights[lane]*magnitudes[lane]*quant;q2+=weights[lane]*quant*quant}if q2>0.0&&qx*qx>best*q2{scale=qx/q2;best=scale*qx;levels=trial}}
			if scale>0.0{let inverse=scale.recip();for group in 0..4{for lane in 0..8{levels[group*8+lane]=qround(0.5*(inverse*magnitudes[group*8+lane]-1.0)).max(0.0).min(2.0)as i8}iq2_nearest(&IQ2_XXS,2,&mut levels[group*8..group*8+8],&magnitudes[group*8..group*8+8],&weights[group*8..group*8+8].iter().map(|value|value.sqrt()).collect::<Vec<_>>(),scale);}let(mut qx,mut q2)=(0.0,0.0);for lane in 0..32{let quant=f32::from(2*levels[lane]+1);qx+=weights[lane]*magnitudes[lane]*quant;q2+=weights[lane]*quant*quant}if q2>0.0{scale=qx/q2}}if scale<0.0{scale=-scale;for sign in &mut signs{*sign=(!*sign)&127}}
			for group in 0..4{packed[block*8+group]=iq2_nearest(&IQ2_XXS,2,&mut levels[group*8..group*8+8],&magnitudes[group*8..group*8+8],&weights[group*8..group*8+8].iter().map(|value|value.sqrt()).collect::<Vec<_>>(),scale)as u8}let word=u32::from(signs[0])|u32::from(signs[1])<<7|u32::from(signs[2])<<14|u32::from(signs[3])<<21;packed[block*8+4..block*8+8].copy_from_slice(&word.to_le_bytes());scales[block]=scale;maximum=maximum.max(scale)}
		if maximum==0.0{put_half(&mut output,0.0);output.extend(packed);continue}let scale=maximum/31.0;for block in 0..8{let code=qround(0.5*(scales[block]/scale-1.0)).max(0.0).min(15.0)as u32;let mut word=u32::from_le_bytes(packed[block*8+4..block*8+8].try_into().unwrap());word|=code<<28;packed[block*8+4..block*8+8].copy_from_slice(&word.to_le_bytes())}put_half(&mut output,scale);output.extend(packed)}output
}
#[rustfmt::skip] fn iq2_16(values:&[f32],importance:Option<&[f32]>,xs:bool)->Vec<u8>{
	let grid=if xs{&IQ2_XS[..]}else{&IQ2_S[..]};let shells=if xs{2}else{1};let mut output=Vec::new();for(chunk,values)in values.chunks(256).enumerate(){let value=|index|values.get(index).copied().unwrap_or(0.0);let importance=|index|importance.and_then(|values|values.get(chunk*256+index)).copied().unwrap_or(0.0);let sigma2=(if xs{1.0}else{2.0})*(0..256).map(|index|value(index)*value(index)).sum::<f32>()/256.0;let mut packed=vec![0_u8;if xs{72}else{80}];let mut scales=[0.0_f32;16];let mut maximum=0.0_f32;
		for block in 0..16{let x=(0..16).map(|offset|value(block*16+offset)).collect::<Vec<_>>();let weights=x.iter().enumerate().map(|(offset,value)|if xs{importance(block*16+offset)*(sigma2+value*value).sqrt()}else{0.25*sigma2+value*value}).collect::<Vec<_>>();let mut magnitudes=x.iter().map(|value|value.abs()).collect::<Vec<_>>();let mut signs=[0_u8;2];for group in 0..2{let mut flips=0;for lane in 0..8{if x[group*8+lane]<0.0{flips+=1;signs[group]|=1<<lane}}if xs&&flips%2!=0{let lane=(0..8).min_by(|a,b|(weights[group*8+*a]*x[group*8+*a]*x[group*8+*a]).total_cmp(&(weights[group*8+*b]*x[group*8+*b]*x[group*8+*b]))).unwrap();magnitudes[group*8+lane]=-magnitudes[group*8+lane];signs[group]^=1<<lane}if xs{signs[group]&=127}}let max=magnitudes.iter().copied().fold(0.0_f32,f32::max);if max<if xs{1.0e-15}else{1.0e-8}{continue}let mut best=0.0_f32;let mut scale=max/5.0;let mut levels=[0_i8;16];let mut on_grid=[true;2];
			for step in -9..=9{let inverse=(5.0+0.1*step as f32)/max;let trial_scale=inverse.recip();let mut trial=[0_i8;16];let mut trial_on=[true;2];for group in 0..2{for lane in 0..8{trial[group*8+lane]=qround(0.5*(inverse*magnitudes[group*8+lane]-1.0)).max(0.0).min(2.0)as i8}let key=(0..8).fold(0_u16,|key,lane|key|(trial[group*8+lane]as u16)<<(2*lane));trial_on[group]=grid.contains(&key);iq2_nearest(grid,shells,&mut trial[group*8..group*8+8],&magnitudes[group*8..group*8+8],&weights[group*8..group*8+8].iter().map(|value|value.sqrt()).collect::<Vec<_>>(),trial_scale);}let(mut qx,mut q2)=(0.0,0.0);for lane in 0..16{let quant=f32::from(2*trial[lane]+1);qx+=weights[lane]*magnitudes[lane]*quant;q2+=weights[lane]*quant*quant}if q2>0.0&&qx*qx>best*q2{scale=qx/q2;best=scale*qx;levels=trial;on_grid=trial_on}}
			if on_grid.iter().any(|value|!*value)&&scale>0.0{let inverse=scale.recip();for group in 0..2{if on_grid[group]{continue}for lane in 0..8{levels[group*8+lane]=qround(0.5*(inverse*magnitudes[group*8+lane]-1.0)).max(0.0).min(2.0)as i8}iq2_nearest(grid,shells,&mut levels[group*8..group*8+8],&magnitudes[group*8..group*8+8],&weights[group*8..group*8+8].iter().map(|value|value.sqrt()).collect::<Vec<_>>(),scale);}let(mut qx,mut q2)=(0.0,0.0);for lane in 0..16{let quant=f32::from(2*levels[lane]+1);qx+=weights[lane]*magnitudes[lane]*quant;q2+=weights[lane]*quant*quant}if q2>0.0{scale=qx/q2}}if scale<0.0{scale=-scale;for sign in &mut signs{*sign=if xs{(!*sign)&127}else{!*sign}}}
			for group in 0..2{let index=iq2_nearest(grid,shells,&mut levels[group*8..group*8+8],&magnitudes[group*8..group*8+8],&weights[group*8..group*8+8].iter().map(|value|value.sqrt()).collect::<Vec<_>>(),scale);let slot=2*block+group;if xs{let word=index as u16|u16::from(signs[group])<<9;packed[2*slot..2*slot+2].copy_from_slice(&word.to_le_bytes())}else{packed[slot]=index as u8;packed[64+slot/4]|=((index>>8)as u8)<<(2*(slot%4));packed[32+slot]=signs[group]}}scales[block]=scale;maximum=maximum.max(scale)}
		if maximum==0.0{put_half(&mut output,0.0);output.extend(packed);continue}let scale=maximum/31.0;let offset=if xs{64}else{72};for block in 0..16{let code=qround(0.5*(scales[block]/scale-1.0)).max(0.0).min(15.0)as u8;packed[offset+block/2]|=code<<(block%2*4)}put_half(&mut output,scale*if xs{1.0}else{0.9875});output.extend(packed)}output
} #[rustfmt::skip] fn iq3_xxs(values: &[f32]) -> Vec<u8> {
	let mut output = Vec::new(); for values in values.chunks(256) {
		let value = |index| values.get(index).copied().unwrap_or(0.0); let mut packed = [0_u8; 96]; let mut scales = [0.0_f32; 8]; let mut maximum = 0.0_f32;
		for block in 0..8 { let x = (0..32).map(|offset| value(block * 32 + offset)).collect::<Vec<_>>(); let weights = x.iter().map(|value| value * value).collect::<Vec<_>>(); let mut magnitudes = x.iter().map(|value| value.abs()).collect::<Vec<_>>(); let mut signs = [0_u8; 4];
			for group in 0..4 { let mut flips = 0; for lane in 0..8 { if x[group * 8 + lane] < 0.0 { flips += 1; signs[group] |= 1 << lane } } if flips % 2 != 0 { let lane = (0..8).min_by(|a,b| (weights[group*8+*a]*x[group*8+*a]*x[group*8+*a]).total_cmp(&(weights[group*8+*b]*x[group*8+*b]*x[group*8+*b]))).unwrap(); magnitudes[group*8+lane] = -magnitudes[group*8+lane]; signs[group] ^= 1 << lane } signs[group] &= 127 }
			let max = magnitudes.iter().copied().fold(0.0_f32, f32::max); if max < 1.0e-6 { continue }
			let mut best = 0.0_f32; let mut scale = max / 15.0; let mut levels = [0_i8; 32];
			for step in -15..=15 { let inverse = (15.0 + 0.2 * step as f32) / max; let trial_scale = inverse.recip(); let mut trial = [0_i8; 32]; for group in 0..8 { for lane in 0..4 { trial[group*4+lane] = qround(0.5*(inverse*magnitudes[group*4+lane]-1.0)).max(0.0).min(7.0) as i8 } iq3_nearest(&IQ3_XXS, &mut trial[group*4..group*4+4], &magnitudes[group*4..group*4+4], &weights[group*4..group*4+4].iter().map(|value| value.sqrt()).collect::<Vec<_>>(), trial_scale); } let (mut qx, mut q2) = (0.0,0.0); for lane in 0..32 { let quant = f32::from(2*trial[lane]+1); qx += weights[lane]*magnitudes[lane]*quant; q2 += weights[lane]*quant*quant } if q2 > 0.0 && qx*qx > best*q2 { scale=qx/q2; best=scale*qx; levels=trial } }
			for group in 0..8 { packed[block*8+group] = iq3_nearest(&IQ3_XXS, &mut levels[group*4..group*4+4], &magnitudes[group*4..group*4+4], &weights[group*4..group*4+4].iter().map(|value| value.sqrt()).collect::<Vec<_>>(), scale) as u8 }
			let word = u32::from(signs[0]) | u32::from(signs[1])<<7 | u32::from(signs[2])<<14 | u32::from(signs[3])<<21; packed[64+block*4..68+block*4].copy_from_slice(&word.to_le_bytes()); scales[block]=scale; maximum=maximum.max(scale)
		} if maximum == 0.0 { put_half(&mut output, 0.0); output.extend(packed); continue }
		let scale = maximum / 31.0; for block in 0..8 { let code=qround(0.5*(scales[block]/scale-1.0)).max(0.0).min(15.0) as u32; let mut word=u32::from_le_bytes(packed[64+block*4..68+block*4].try_into().unwrap()); word|=code<<28; packed[64+block*4..68+block*4].copy_from_slice(&word.to_le_bytes()) }
		put_half(&mut output, scale * 1.0125); output.extend(packed)
	} output
}
#[rustfmt::skip] fn iq3_s(values: &[f32]) -> Vec<u8> {
	let mut output=Vec::new(); for values in values.chunks(256) {
		let value=|index| values.get(index).copied().unwrap_or(0.0); let mut packed=[0_u8;108]; let mut scales=[0.0_f32;8]; let mut maximum=0.0_f32;
		for block in 0..8 { let x=(0..32).map(|offset| value(block*32+offset)).collect::<Vec<_>>(); let weights=x.iter().map(|value| value*value).collect::<Vec<_>>(); let magnitudes=x.iter().map(|value| value.abs()).collect::<Vec<_>>(); let max=magnitudes.iter().copied().fold(0.0_f32,f32::max); if max==0.0 {continue} let mut best=0.0_f32; let mut scale=max/15.0; let mut levels=[0_i8;32];
			for step in -9..=9 { let inverse=(15.0+0.2*step as f32)/max; let trial_scale=inverse.recip(); let mut trial=[0_i8;32]; for group in 0..8 { for lane in 0..4 {trial[group*4+lane]=qround(0.5*(inverse*magnitudes[group*4+lane]-1.0)).max(0.0).min(7.0) as i8} iq3_nearest(&IQ3_S,&mut trial[group*4..group*4+4],&magnitudes[group*4..group*4+4],&weights[group*4..group*4+4].iter().map(|value|value.sqrt()).collect::<Vec<_>>(),trial_scale); } let(mut qx,mut q2)=(0.0,0.0); for lane in 0..32 {let quant=f32::from(2*trial[lane]+1);qx+=weights[lane]*magnitudes[lane]*quant;q2+=weights[lane]*quant*quant} if q2>0.0&&qx*qx>best*q2 {scale=qx/q2;best=scale*qx;levels=trial} }
			for group in 0..8 {let index=iq3_nearest(&IQ3_S,&mut levels[group*4..group*4+4],&magnitudes[group*4..group*4+4],&weights[group*4..group*4+4].iter().map(|value|value.sqrt()).collect::<Vec<_>>(),scale);packed[block*8+group]=index as u8;packed[64+(block*8+group)/8]|=((index>>8)as u8)<<((block*8+group)%8)} for group in 0..4 {packed[72+block*4+group]=(0..8).fold(0,|signs,lane|signs|u8::from(x[group*8+lane]<0.0)<<lane)} scales[block]=scale;maximum=maximum.max(scale)
		}
		if maximum==0.0 {put_half(&mut output,0.0);output.extend(packed);continue} let scale=maximum/31.0; for pair in 0..4 {let low=qround(0.5*(scales[pair*2]/scale-1.0)).max(0.0).min(15.0)as u8;let high=qround(0.5*(scales[pair*2+1]/scale-1.0)).max(0.0).min(15.0)as u8;packed[104+pair]=low|high<<4} put_half(&mut output,scale*1.033);output.extend(packed)
	} output
}
fn iq4_code(value: f32) -> u8 { IQ4.iter().enumerate().min_by(|left, right| (value - f32::from(*left.1)).abs().total_cmp(&(value - f32::from(*right.1)).abs())).unwrap().0 as u8 }
#[rustfmt::skip]
fn iq4_fit(values: &[f32], tries: i32) -> (f32, Vec<u8>) {
	let mut extreme = 0.0_f32;
	for value in values { if value.abs() > extreme.abs() { extreme = *value } }
	if extreme.abs() < 1.0e-15 { return (0.0, vec![0; values.len()]) }
	let initial = if tries > 0 { -extreme / f32::from(IQ4[0]) } else { extreme / f32::from(IQ4[0]) };
	let score = |inverse: f32| {
		values.iter().map(|value| { let quant = f32::from(IQ4[usize::from(iq4_code(value * inverse))]);
			(value * value * quant * value, value * value * quant * quant) }).fold((0.0, 0.0), |left, right| (left.0 + right.0, left.1 + right.1))
	};
	let (numerator, denominator) = score(initial.recip());
	let mut scale = if denominator > 0.0 { numerator / denominator } else { 0.0 };
	let mut best = scale * numerator;
	for attempt in -tries..=tries {
		let (numerator, denominator) = score((attempt as f32 + f32::from(IQ4[0])) / extreme);
		if denominator > 0.0 && numerator * numerator > best * denominator { scale = numerator / denominator; best = scale * numerator }
	}
	let inverse = if tries > 0 && scale != 0.0 { scale.recip() } else { initial.recip() };
	(scale, values.iter().map(|value| iq4_code(value * inverse)).collect())
}
#[derive(Clone, Copy)]
struct IntegerFormat(u16);
impl IntegerFormat {
	fn selection(self)->Option<u16>{let(family,bits,variant)=(self.0>>12,self.bits(),self.0>>8&15);match(family,bits,variant){(0,2|6|8,3)=>Some(3),(0,3|4|5,3|5)=>Some(5),(0,3|4|5,4)=>Some(4),(0,3,6)=>Some(6),(1,2,4)|(1,3,2|4)=>Some(variant),_=>None}}
	fn tensor(self,role:u8,more:bool,output:bool)->u16{let(family,bits,style)=(self.0>>12,self.bits(),self.selection().unwrap());if output{return 3<<8|6}if family==1{return match(bits,style,role,more){(2,4,2|3,_)|(2,4,_,true)=>1<<12|3<<8|3,(3,2,0|1,_)|(3,2,_,false)=>1<<12|1<<8|3,(3,4,2|3,_)|(3,4,_,true)=>3<<8|4,_=>1<<12|3<<8|u16::from(bits)}}let bits=match(bits,style,role){(2,_,2|3)=>3,(3,5,2)=>5,(3,5,3)=>4,(3,6,2|3)=>5,(4,4,2)=>5,(4,5,2)if more=>6,(5,5,2)if more=>6,_=>bits};3<<8|u16::from(bits)}
}
trait Integer {
	fn compress(self, weights: &[f64], importance: &[f64], config: Config) -> Result<(Vec<u8>, Vec<f64>)>;
	fn decompress(self, data: &[u8], codebook: &[f64], count: usize) -> Result<Vec<f64>>;
	fn bits(self) -> u8;
}
impl Integer for IntegerFormat {
	fn bits(self) -> u8 {
		self.0 as u8
	}
	fn compress(self, weights: &[f64], importance: &[f64], config: Config) -> Result<(Vec<u8>, Vec<f64>)> {
		let bits = self.bits();
		let (family, variant) = (self.0 >> 12, self.0 >> 8 & 15);
		if family == 0 && variant < 2 && matches!(bits, 4 | 5 | 8) {
			let block = 32;
			let mut data = Vec::new();
			for values in weights.chunks(block) {
				let value = |index| values.get(index).copied().unwrap_or(0.0) as f32;
				let (minimum, maximum) = (0..block).map(value).fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), value| (low.min(value), high.max(value)));
				let extreme = (0..block).map(value).max_by(|a, b| a.abs().total_cmp(&b.abs())).unwrap_or(0.0);
				let scale = match (bits, variant) {
					(8, _) => extreme.abs() / 127.0,
					(_, 0) => extreme / -(1_i32 << (bits - 1)) as f32,
					(_, 1) => (maximum - minimum) / ((1_u16 << bits) - 1) as f32,
					_ => unreachable!(),
				};
				let inverse = if scale == 0.0 { 0.0 } else { scale.recip() };
				put_half(&mut data, scale);
				if variant == 1 && bits != 8 {
					put_half(&mut data, minimum)
				}
				let (mut low, mut high) = ([0_u8; 32], [0_u8; 4]);
				let mut sum = 0_i32;
				for index in 0..block {
					let shifted = match (bits, variant) {
						(8, _) => (value(index) * inverse).round() + 128.0,
						(_, 0) => value(index) * inverse + (1_i32 << (bits - 1)) as f32 + 0.5,
						(_, 1) => (value(index) - minimum) * inverse + 0.5,
						_ => unreachable!(),
					};
					let code = shifted.max(0.0).min(f32::from((1_u16 << bits) - 1)) as u8;
					if bits == 4 || bits == 5 {
						low[index % 16] |= (code & 15) << (index / 16 * 4)
					}
					if bits == 5 {
						high[index / 8] |= (code >> 4) << (index % 8)
					}
					if bits == 8 {
						low[index] = code.wrapping_sub(128);
						sum += i32::from(i8::from_ne_bytes([low[index]]))
					}
				}
				if bits == 5 {
					data.extend(high)
				}
				if bits == 8 && variant == 1 {
					put_half(&mut data, scale * sum as f32)
				}
				data.extend_from_slice(
					&low[..match bits {
						4 | 5 => 16,
						8 => 32,
						_ => unreachable!(),
					}],
				);
			}
			return Ok((data, Vec::new()));
		}
		if family == 0 && variant == 3 && bits == 2 {
			let mut data = Vec::new();
			for values in weights.chunks(256) {
				let values = (0..256).map(|index| values.get(index).copied().unwrap_or(0.0) as f32).collect::<Vec<_>>();
				let (mut codes, mut scales, mut minima) = ([0_u8; 256], [0.0_f32; 16], [0.0_f32; 16]);
				for block in 0..16 {
					let weights = values[block * 16..block * 16 + 16].iter().map(|value| value.abs()).collect::<Vec<_>>();
					(scales[block], minima[block]) = qkx2(&values[block * 16..block * 16 + 16], &weights, 3, (-0.5, 0.1, 15), true, &mut codes[block * 16..block * 16 + 16]);
				}
				let (max_scale, max_minimum) = (positive_max(&scales), positive_max(&minima));
				let (scale, minimum) = (max_scale / 15.0, max_minimum / 15.0);
				let (stored_scale, stored_minimum) = (unfp16(fp16(scale)), unfp16(fp16(minimum)));
				let mut packed_scales = [0_u8; 16];
				for block in 0..16 {
					let scale_code = if max_scale > 0.0 { qround(15.0 * scales[block] / max_scale) as u8 } else { 0 };
					let minimum_code = if max_minimum > 0.0 { qround(15.0 * minima[block] / max_minimum) as u8 } else { 0 };
					packed_scales[block] = scale_code | minimum_code << 4;
					let (d, m) = (stored_scale * f32::from(scale_code), stored_minimum * f32::from(minimum_code));
					if d != 0.0 {
						for offset in 0..16 {
							codes[block * 16 + offset] = qround((values[block * 16 + offset] + m) / d).max(0.0).min(3.0) as u8;
						}
					}
				}
				let mut packed = [0_u8; 64];
				for group in (0..256).step_by(128) {
					for offset in 0..32 {
						packed[group / 4 + offset] = codes[group + offset] | codes[group + offset + 32] << 2 | codes[group + offset + 64] << 4 | codes[group + offset + 96] << 6;
					}
				}
				data.extend(packed_scales);
				data.extend(packed);
				put_half(&mut data, scale);
				put_half(&mut data, minimum);
			}
			return Ok((data, Vec::new()));
		}
		if family == 0 && variant == 3 && bits == 3 {
			let mut data = Vec::new();
			for values in weights.chunks(256) {
				let values = (0..256).map(|index| values.get(index).copied().unwrap_or(0.0) as f32).collect::<Vec<_>>();
				let (mut codes, mut block_scales) = ([0_i8; 256], [0.0_f32; 16]);
				let (mut maximum, mut extreme) = (0.0, 0.0);
				for block in 0..16 {
					block_scales[block] = q3(&values[block * 16..block * 16 + 16], &mut codes[block * 16..block * 16 + 16]);
					if block_scales[block].abs() > extreme {
						extreme = block_scales[block].abs();
						maximum = block_scales[block]
					}
				}
				let inverse = if maximum == 0.0 { 0.0 } else { -32.0 / maximum };
				let scale = if inverse == 0.0 { 0.0 } else { inverse.recip() };
				let stored_scale = unfp16(fp16(scale));
				let mut scales = [0_u8; 12];
				for block in 0..16 {
					let mut code = qround(inverse * block_scales[block]).max(-32.0).min(31.0) as i8 + 32;
					if block < 8 {
						scales[block] = code as u8 & 15
					} else {
						scales[block - 8] |= (code as u8 & 15) << 4
					}
					code >>= 4;
					scales[block % 4 + 8] |= (code as u8) << (2 * (block / 4));
					let signed = ((scales[if block < 8 { block } else { block - 8 }] >> if block < 8 { 0 } else { 4 } & 15) | ((scales[8 + block % 4] >> (2 * (block / 4)) & 3) << 4)) as i8 - 32;
					let d = stored_scale * f32::from(signed);
					if d != 0.0 {
						for offset in 0..16 {
							codes[block * 16 + offset] = qround(values[block * 16 + offset] / d).max(-4.0).min(3.0) as i8 + 4;
						}
					}
				}
				let (mut high, mut low) = ([0_u8; 32], [0_u8; 64]);
				for index in 0..256 {
					let mut code = codes[index] as u8;
					if code > 3 {
						high[index % 32] |= 1 << (index / 32);
						code -= 4
					}
					low[index / 128 * 32 + index % 32] |= code << (index % 128 / 32 * 2);
				}
				data.extend(high);
				data.extend(low);
				data.extend(scales);
				put_half(&mut data, scale);
			}
			return Ok((data, Vec::new()));
		}
		if family == 0 && variant == 3 && matches!(bits, 4 | 5) {
			let mut data = Vec::new();
			for values in weights.chunks(256) {
				let values = (0..256).map(|index| values.get(index).copied().unwrap_or(0.0) as f32).collect::<Vec<_>>();
				let (mut codes, mut block_scales, mut minima) = ([0_u8; 256], [0.0_f32; 8], [0.0_f32; 8]);
				for block in 0..8 {
					let slice = &values[block * 32..block * 32 + 32];
					let rms = (slice.iter().map(|value| value * value).sum::<f32>() / 32.0).sqrt();
					let weights = slice.iter().map(|value| rms + value.abs()).collect::<Vec<_>>();
					let (levels, range) = if bits == 4 { (15, (-1.0, 0.1, 20)) } else { (31, (-0.5, 0.1, 15)) };
					(block_scales[block], minima[block]) = qkx2(slice, &weights, levels, range, false, &mut codes[block * 32..block * 32 + 32]);
				}
				let (maximum, max_minimum) = (positive_max(&block_scales), positive_max(&minima));
				let (scale, minimum) = (maximum / 63.0, max_minimum / 63.0);
				let (stored_scale, stored_minimum) = (unfp16(fp16(scale)), unfp16(fp16(minimum)));
				let mut metadata = [0_u8; 12];
				for block in 0..8 {
					let scale_code = if maximum > 0.0 { qround(63.0 * block_scales[block] / maximum).min(63.0) as u8 } else { 0 };
					let minimum_code = if max_minimum > 0.0 { qround(63.0 * minima[block] / max_minimum).min(63.0) as u8 } else { 0 };
					if block < 4 {
						metadata[block] = scale_code;
						metadata[block + 4] = minimum_code
					} else {
						metadata[block + 4] = scale_code & 15 | (minimum_code & 15) << 4;
						metadata[block - 4] |= scale_code >> 4 << 6;
						metadata[block] |= minimum_code >> 4 << 6
					}
				}
				for block in 0..8 {
					let (scale_code, minimum_code) = k_scale(&metadata, block);
					let (d, m) = (stored_scale * f32::from(scale_code), stored_minimum * f32::from(minimum_code));
					if d != 0.0 {
						for offset in 0..32 {
							codes[block * 32 + offset] = qround((values[block * 32 + offset] + m) / d).max(0.0).min(if bits == 4 { 15.0 } else { 31.0 }) as u8;
						}
					}
				}
				let (mut high, mut packed) = ([0_u8; 32], [0_u8; 128]);
				for group in (0..256).step_by(64) {
					for offset in 0..32 {
						packed[group / 2 + offset] = codes[group + offset] & 15 | (codes[group + offset + 32] & 15) << 4;
						high[offset] |= (codes[group + offset] >> 4) << (group / 32) | (codes[group + offset + 32] >> 4) << (group / 32 + 1)
					}
				}
				put_half(&mut data, scale);
				put_half(&mut data, minimum);
				data.extend(metadata);
				if bits == 5 {
					data.extend(high)
				}
				data.extend(packed);
			}
			return Ok((data, Vec::new()));
		}
		if family == 0 && variant == 3 && bits == 6 {
			let mut data = Vec::new();
			for values in weights.chunks(256) {
				let values = (0..256).map(|index| values.get(index).copied().unwrap_or(0.0) as f32).collect::<Vec<_>>();
				let (mut codes, mut block_scales) = ([0_i8; 256], [0.0_f32; 16]);
				let (mut maximum, mut extreme) = (0.0, 0.0);
				for block in 0..16 {
					block_scales[block] = qx(&values[block * 16..block * 16 + 16], 32, &mut codes[block * 16..block * 16 + 16]);
					if block_scales[block].abs() > extreme {
						extreme = block_scales[block].abs();
						maximum = block_scales[block]
					}
				}
				let inverse = if extreme < 1.0e-15 { 0.0 } else { -128.0 / maximum };
				let scale = if inverse == 0.0 { 0.0 } else { inverse.recip() };
				let stored_scale = unfp16(fp16(scale));
				let mut scales = [0_i8; 16];
				for block in 0..16 {
					scales[block] = qround(inverse * block_scales[block]).min(127.0) as i8;
					let d = stored_scale * f32::from(scales[block]);
					if d != 0.0 {
						for offset in 0..16 {
							codes[block * 16 + offset] = qround(values[block * 16 + offset] / d).max(-32.0).min(31.0) as i8 + 32;
						}
					}
				}
				let (mut low, mut high) = ([0_u8; 128], [0_u8; 64]);
				for group in (0..256).step_by(128) {
					for offset in 0..32 {
						let code = [codes[group + offset], codes[group + offset + 32], codes[group + offset + 64], codes[group + offset + 96]].map(|value| value as u8);
						low[group / 2 + offset] = code[0] & 15 | (code[2] & 15) << 4;
						low[group / 2 + offset + 32] = code[1] & 15 | (code[3] & 15) << 4;
						high[group / 4 + offset] = code[0] >> 4 | code[1] >> 4 << 2 | code[2] >> 4 << 4 | code[3] >> 4 << 6;
					}
				}
				data.extend(low);
				data.extend(high);
				data.extend(scales.map(|value| value as u8));
				put_half(&mut data, scale);
			}
			return Ok((data, Vec::new()));
		}
		if family == 0 && variant == 3 && bits == 8 {
			let mut data = Vec::new();
			for values in weights.chunks(256) {
				let value = |index| values.get(index).copied().unwrap_or(0.0) as f32;
				let maximum = (0..256).map(value).max_by(|a, b| a.abs().total_cmp(&b.abs())).unwrap_or(0.0);
				let inverse = if maximum == 0.0 { 0.0 } else { -127.0 / maximum }; let scale = if inverse == 0.0 { 0.0 } else { inverse.recip() };
				data.extend(scale.to_le_bytes()); let codes = (0..256).map(|index| qround(inverse * value(index)).max(-128.0).min(127.0) as i8).collect::<Vec<_>>();
				data.extend(codes.iter().map(|code| *code as u8)); for block in codes.chunks(16) { data.extend(block.iter().map(|code| i16::from(*code)).sum::<i16>().to_le_bytes()) }
			}
			return Ok((data, Vec::new()));
		}
		if family == 0 && variant == 2 && bits == 4 {
			const NF4: [f64; 16] = [-1.0, -0.6961928009986877, -0.5250730514526367, -0.39491748809814453, -0.28444138169288635, -0.18477343022823334, -0.09105003625154495, 0.0, 0.07958029955625534, 0.16093020141124725, 0.24611230194568634, 0.33791524171829224, 0.44070982933044434, 0.5626170039176941, 0.7229568362236023, 1.0];
			let mut metadata = vec![config.quantization_block as f64];
			metadata.extend(NF4);
			let mut data = vec![0_u8; weights.len().div_ceil(2)];
			for (block, values) in weights.chunks(config.quantization_block).enumerate() {
				let scale = values.iter().map(|value| value.abs()).max_by(f64::total_cmp).unwrap_or(0.0);
				metadata.push(scale);
				for (offset, weight) in values.iter().enumerate() {
					let index = block * config.quantization_block + offset;
					let code = nearest(std::slice::from_ref(&(if scale == 0.0 { 0.0 } else { weight / scale })), &NF4, 1).0 as u8;
					data[index / 2] |= code << (index % 2 * 4);
				}
			}
			return Ok((data, metadata));
		}
		if family == 1 && variant == 5 && bits == 4 {
			let mut data = Vec::new();
			for values in weights.chunks(32) {
				let values = (0..32).map(|index| values.get(index).copied().unwrap_or(0.0) as f32).collect::<Vec<_>>();
				let (scale, codes) = iq4_fit(&values, -1);
				put_half(&mut data, scale);
				for index in 0..16 {
					data.push(codes[index] | codes[index + 16] << 4)
				}
			}
			return Ok((data, Vec::new()));
		}
		if family == 1 && variant == 2 && bits == 4 {
			let mut data = Vec::new();
			for values in weights.chunks(256) {
				let values = (0..256).map(|index| values.get(index).copied().unwrap_or(0.0) as f32).collect::<Vec<_>>();
				let (mut scales, mut codes) = ([0.0_f32; 8], [0_u8; 256]);
				let (mut maximum, mut extreme) = (0.0, 0.0);
				for block in 0..8 {
					let (scale, fitted) = iq4_fit(&values[block * 32..block * 32 + 32], 7);
					scales[block] = scale;
					codes[block * 32..block * 32 + 32].copy_from_slice(&fitted);
					if scale.abs() > extreme {
						extreme = scale.abs();
						maximum = scale
					}
				}
				let scale = -maximum / 32.0;
				let stored_scale = unfp16(fp16(scale));
				let (mut high, mut low) = (0_u16, [0_u8; 4]);
				for block in 0..8 {
					let signed = if scale == 0.0 { 0 } else { qround(scales[block] / scale).max(-32.0).min(31.0) as i8 };
					let code = (signed + 32) as u8;
					low[block / 2] |= (code & 15) << (block % 2 * 4);
					high |= u16::from(code >> 4) << (block * 2);
					let d = stored_scale * f32::from(signed);
					let inverse = if d == 0.0 { 0.0 } else { d.recip() };
					for offset in 0..32 {
						codes[block * 32 + offset] = iq4_code(values[block * 32 + offset] * inverse)
					}
				}
				put_half(&mut data, scale);
				data.extend(high.to_le_bytes());
				data.extend(low);
				for block in 0..8 {
					for offset in 0..16 {
						data.push(codes[block * 32 + offset] | codes[block * 32 + offset + 16] << 4)
					}
				}
			}
			return Ok((data, Vec::new()));
		}
		if family == 1 && variant == 1 && bits == 2 { require(importance.len()==weights.len()&&importance.iter().all(|value|value.is_finite()&&*value>=0.0)&&importance.iter().any(|value|*value>0.0),"GGML IQ2_XXS requires trained importance weights")?;return Ok((iq2_xxs(&weights.iter().map(|value|*value as f32).collect::<Vec<_>>(),&importance.iter().map(|value|*value as f32).collect::<Vec<_>>()),Vec::new())) }
		if family == 1 && variant == 2 && bits == 2 { require(importance.len()==weights.len()&&importance.iter().all(|value|value.is_finite()&&*value>=0.0)&&importance.iter().any(|value|*value>0.0),"GGML IQ2_XS requires trained importance weights")?;return Ok((iq2_16(&weights.iter().map(|value|*value as f32).collect::<Vec<_>>(),Some(&importance.iter().map(|value|*value as f32).collect::<Vec<_>>()),true),Vec::new())) }
		if family == 1 && matches!(variant,3|4) && bits == 1 { require(importance.len()==weights.len()&&importance.iter().all(|value|value.is_finite()&&*value>=0.0)&&importance.iter().any(|value|*value>0.0),format!("GGML IQ1_{} requires trained importance weights",if variant==3{"S"}else{"M"}))?;return Ok((iq1(&weights.iter().map(|value|*value as f32).collect::<Vec<_>>(),&importance.iter().map(|value|*value as f32).collect::<Vec<_>>(),variant==4),Vec::new())) }
		if family == 1 && variant == 1 && bits == 3 { return Ok((iq3_xxs(&weights.iter().map(|value| *value as f32).collect::<Vec<_>>()), Vec::new())) }
		if family == 1 && variant == 3 && bits == 2 { return Ok((iq2_16(&weights.iter().map(|value|*value as f32).collect::<Vec<_>>(),None,false),Vec::new())) }
		if family == 1 && variant == 3 && bits == 3 { return Ok((iq3_s(&weights.iter().map(|value| *value as f32).collect::<Vec<_>>()), Vec::new())) }
		Err(RecipeError::new(format!("quantization code {} is unavailable; available GGML formats: Q4_0, Q4_1, Q5_0, Q5_1, Q8_0, Q8_1, Q2_K, Q3_K, Q4_K, Q5_K, Q6_K, Q8_K, Q4_NF, IQ1_S, IQ1_M, IQ2_XXS, IQ2_XS, IQ2_S, IQ3_XXS, IQ3_S, IQ4_NL, and IQ4_XS", self.0)))
	}
	fn decompress(self, data: &[u8], codebook: &[f64], count: usize) -> Result<Vec<f64>> {
		let (family, variant, bits) = (self.0 >> 12, self.0 >> 8 & 15, self.bits());
		if family == 0 && variant < 2 && matches!(bits, 4 | 5 | 8) {
			let block = 32;
			let header = if variant == 1 { 4 } else { 2 };
			let payload = match bits {
				4 => 16,
				5 => 20,
				8 => 32,
				_ => unreachable!(),
			};
			let stride = header + payload;
			require(data.len() >= count.div_ceil(block) * stride, "GGML quantized weights are invalid")?;
			let mut weights = Vec::with_capacity(count);
			for bytes in data.chunks_exact(stride) {
				let scale = half(bytes);
				let minimum = if variant == 1 && bits != 8 { half(&bytes[2..]) } else { 0.0 };
				let codes = &bytes[header..header + payload];
				for index in 0..block.min(count - weights.len()) {
					let code = match bits {
						4 => codes[index % 16] >> (index / 16 * 4) & 15,
						5 => (codes[4 + index % 16] >> (index / 16 * 4) & 15) | ((codes[index / 8] >> (index % 8) & 1) << 4),
						8 => codes[index],
						_ => unreachable!(),
					};
					let value = match (bits, variant) {
						(8, _) => i8::from_ne_bytes([code]) as f32 * scale,
						(_, 0) => (i32::from(code) - (1_i32 << (bits - 1))) as f32 * scale,
						(_, 1) => f32::from(code) * scale + minimum,
						_ => unreachable!(),
					};
					weights.push(f64::from(value));
				}
			}
			return Ok(weights);
		}
		if family == 0 && variant == 3 && bits == 2 {
			const STRIDE: usize = 84;
			require(data.len() >= count.div_ceil(256) * STRIDE, "GGML Q2_K weights are invalid")?;
			let mut weights = Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE) {
				let (scale, minimum) = (half(&bytes[80..]), half(&bytes[82..]));
				let packed = &bytes[16..80];
				for group in (0..256).step_by(128) {
					for shift in (0..8).step_by(2) {
						for half in 0..2 {
							let block = group / 16 + shift + half;
							let metadata = bytes[block];
							let (d, m) = (scale * f32::from(metadata & 15), minimum * f32::from(metadata >> 4));
							for offset in 0..16 {
								if weights.len() == count {
									return Ok(weights);
								}
								weights.push(f64::from(d * f32::from(packed[group / 4 + half * 16 + offset] >> shift & 3) - m));
							}
						}
					}
				}
			}
			return Ok(weights);
		}
		if family == 0 && variant == 3 && bits == 3 {
			const STRIDE: usize = 110;
			require(data.len() >= count.div_ceil(256) * STRIDE, "GGML Q3_K weights are invalid")?;
			let mut weights = Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE) {
				let scale = half(&bytes[108..]);
				for index in 0..256 {
					if weights.len() == count {
						return Ok(weights);
					}
					let block = index / 16;
					let low_scale = bytes[96 + if block < 8 { block } else { block - 8 }] >> if block < 8 { 0 } else { 4 } & 15;
					let high_scale = bytes[104 + block % 4] >> (2 * (block / 4)) & 3;
					let block_scale = (low_scale | high_scale << 4) as i8 - 32;
					let low = bytes[32 + index / 128 * 32 + index % 32] >> (index % 128 / 32 * 2) & 3;
					let quant = i8::try_from(low).unwrap() - if bytes[index % 32] >> (index / 32) & 1 == 0 { 4 } else { 0 };
					weights.push(f64::from(scale * f32::from(block_scale) * f32::from(quant)));
				}
			}
			return Ok(weights);
		}
		if family == 0 && variant == 3 && matches!(bits, 4 | 5) {
			let stride = if bits == 4 { 144 } else { 176 };
			require(data.len() >= count.div_ceil(256) * stride, "GGML Q4_K or Q5_K weights are invalid")?;
			let mut weights = Vec::with_capacity(count);
			for bytes in data.chunks_exact(stride) {
				let (scale, minimum) = (half(bytes), half(&bytes[2..]));
				for index in 0..256 {
					if weights.len() == count {
						return Ok(weights);
					}
					let block = index / 32;
					let (scale_code, minimum_code) = k_scale(&bytes[4..16], block);
					let packed = bytes[(if bits == 4 { 16 } else { 48 }) + index / 64 * 32 + index % 32];
					let mut code = if index % 64 < 32 { packed & 15 } else { packed >> 4 };
					if bits == 5 {
						code |= (bytes[16 + index % 32] >> (index / 32) & 1) << 4
					}
					weights.push(f64::from(scale * f32::from(scale_code) * f32::from(code) - minimum * f32::from(minimum_code)));
				}
			}
			return Ok(weights);
		}
		if family == 0 && variant == 3 && bits == 6 {
			const STRIDE: usize = 210;
			require(data.len() >= count.div_ceil(256) * STRIDE, "GGML Q6_K weights are invalid")?;
			let mut weights = Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE) {
				let scale = half(&bytes[208..]);
				for index in 0..256 {
					if weights.len() == count {
						return Ok(weights);
					}
					let group = index / 128 * 128;
					let quarter = index % 128 / 32;
					let packed = bytes[group / 2 + index % 32 + if quarter % 2 == 0 { 0 } else { 32 }];
					let low = if quarter < 2 { packed & 15 } else { packed >> 4 };
					let high = bytes[128 + group / 4 + index % 32] >> (quarter * 2) & 3;
					let quant = (low | high << 4) as i8 - 32;
					let block_scale = i8::from_ne_bytes([bytes[192 + index / 16]]);
					weights.push(f64::from(scale * f32::from(block_scale) * f32::from(quant)));
				}
			}
			return Ok(weights);
		}
		if family == 0 && variant == 3 && bits == 8 {
			const STRIDE: usize = 292; require(data.len() >= count.div_ceil(256) * STRIDE, "GGML Q8_K weights are invalid")?;
			let mut weights = Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE) { let scale = float(bytes); for code in &bytes[4..260] { if weights.len() == count { return Ok(weights) } weights.push(f64::from(scale * f32::from(i8::from_ne_bytes([*code])))) } }
			return Ok(weights);
		}
		if self.0 >> 12 == 0 && self.0 >> 8 & 15 == 2 {
			let block = codebook.first().copied().unwrap_or(0.0) as usize;
			require(block != 0 && codebook.len() >= 17 + count.div_ceil(block) && data.len() * 2 >= count, "NF4 weights are invalid")?;
			return (0..count).map(|index| codebook.get(1 + usize::from(data[index / 2] >> (index % 2 * 4) & 15)).zip(codebook.get(17 + index / block)).map(|(value, scale)| value * scale).ok_or_else(|| RecipeError::new("NF4 weight index is invalid"))).collect();
		}
		if family == 1 && variant == 5 && bits == 4 {
			const STRIDE: usize = 18;
			require(data.len() >= count.div_ceil(32) * STRIDE, "GGML IQ4_NL weights are invalid")?;
			let mut weights = Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE) {
				let scale = half(bytes);
				for index in 0..32 {
					if weights.len() == count {
						return Ok(weights);
					}
					let code = if index < 16 { bytes[2 + index] & 15 } else { bytes[2 + index - 16] >> 4 };
					weights.push(f64::from(scale * f32::from(IQ4[usize::from(code)])));
				}
			}
			return Ok(weights);
		}
		if family == 1 && variant == 2 && bits == 4 {
			const STRIDE: usize = 136;
			require(data.len() >= count.div_ceil(256) * STRIDE, "GGML IQ4_XS weights are invalid")?;
			let mut weights = Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE) {
				let (scale, high) = (half(bytes), u16::from_le_bytes([bytes[2], bytes[3]]));
				for index in 0..256 {
					if weights.len() == count {
						return Ok(weights);
					}
					let block = index / 32;
					let scale_code = (bytes[4 + block / 2] >> (block % 2 * 4) & 15) | ((high >> (block * 2) & 3) as u8) << 4;
					let packed = bytes[8 + block * 16 + index % 16];
					let code = if index % 32 < 16 { packed & 15 } else { packed >> 4 };
					weights.push(f64::from(scale * f32::from(scale_code as i8 - 32) * f32::from(IQ4[usize::from(code)])));
				}
			}
			return Ok(weights);
		}
		if family == 1 && variant == 1 && bits == 3 {
			const STRIDE: usize = 98; require(data.len() >= count.div_ceil(256) * STRIDE, "GGML IQ3_XXS weights are invalid")?; let mut weights = Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE) { let scale=half(bytes); for block in 0..8 { let word=u32::from_le_bytes(bytes[66+block*4..70+block*4].try_into().unwrap()); let d=scale*(0.5+(word>>28) as f32)*0.5; for group in 0..4 { let signs=(word>>(7*group)&127) as u8; let signs=signs | ((signs.count_ones() as u8 & 1)<<7); for lane in 0..8 { if weights.len()==count { return Ok(weights) } let grid=usize::from(bytes[2+block*8+group*2+lane/4]); let magnitude=f32::from(iq3_grid(&IQ3_XXS,grid,lane%4)); weights.push(f64::from(d*magnitude*if signs>>lane&1!=0 {-1.0} else {1.0})) } } } }
			return Ok(weights);
		}
		if family==1&&variant==1&&bits==2{
			const STRIDE:usize=66;require(data.len()>=count.div_ceil(256)*STRIDE,"GGML IQ2_XXS weights are invalid")?;let mut weights=Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE){let scale=half(bytes);for block in 0..8{let word=u32::from_le_bytes(bytes[6+block*8..10+block*8].try_into().unwrap());let d=scale*(0.5+(word>>28)as f32)*0.25;for group in 0..4{let signs=(word>>(7*group)&127)as u8;let signs=signs|((signs.count_ones()as u8&1)<<7);let grid=usize::from(bytes[2+block*8+group]);for lane in 0..8{if weights.len()==count{return Ok(weights)}let sign=if signs>>lane&1!=0{-1.0}else{1.0};weights.push(f64::from(d*f32::from(iq2_grid(&IQ2_XXS,grid,lane))*sign))}}}}return Ok(weights)
		}
		if family == 1 && variant == 3 && bits == 2 {
			const STRIDE:usize=82;require(data.len()>=count.div_ceil(256)*STRIDE,"GGML IQ2_S weights are invalid")?;let mut weights=Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE){let scale=half(bytes);for index in 0..256{if weights.len()==count{return Ok(weights)}let block=index/16;let slot=index/8;let grid=usize::from(bytes[2+slot])|usize::from(bytes[66+slot/4]>>(2*(slot%4))&3)<<8;let code=bytes[74+block/2]>>(4*(block%2))&15;let d=scale*(0.5+f32::from(code))*0.25;let sign=if bytes[34+slot]>>(index%8)&1!=0{-1.0}else{1.0};weights.push(f64::from(d*f32::from(iq2_grid(&IQ2_S,grid,index%8))*sign))}}return Ok(weights)
		}
		if family==1&&variant==2&&bits==2{
			const STRIDE:usize=74;require(data.len()>=count.div_ceil(256)*STRIDE,"GGML IQ2_XS weights are invalid")?;let mut weights=Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE){let scale=half(bytes);for index in 0..256{if weights.len()==count{return Ok(weights)}let block=index/16;let slot=index/8;let word=u16::from_le_bytes(bytes[2+2*slot..4+2*slot].try_into().unwrap());let grid=usize::from(word&511);let signs=(word>>9)as u8;let signs=signs|((signs.count_ones()as u8&1)<<7);let code=bytes[66+block/2]>>(4*(block%2))&15;let d=scale*(0.5+f32::from(code))*0.25;let sign=if signs>>(index%8)&1!=0{-1.0}else{1.0};weights.push(f64::from(d*f32::from(iq2_grid(&IQ2_XS,grid,index%8))*sign))}}return Ok(weights)
		}
		if family==1&&variant==3&&bits==1{
			const STRIDE:usize=50;require(data.len()>=count.div_ceil(256)*STRIDE,"GGML IQ1_S weights are invalid")?;let mut weights=Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE){let scale=half(bytes);for index in 0..256{if weights.len()==count{return Ok(weights)}let block=index/32;let group=index/8;let high=u16::from_le_bytes(bytes[34+2*block..36+2*block].try_into().unwrap());let grid=usize::from(bytes[2+4*block+group])|usize::from(high>>(3*group)&7)<<8;let d=scale*f32::from(2*((high>>12)&7)+1);let delta=if high&0x8000!=0{-0.125}else{0.125};weights.push(f64::from(d*(f32::from(iq1_level(grid,index%8))-1.0+delta)))}}return Ok(weights)
		}
		if family==1&&variant==4&&bits==1{
			const STRIDE:usize=56;require(data.len()>=count.div_ceil(256)*STRIDE,"GGML IQ1_M weights are invalid")?;let mut weights=Vec::with_capacity(count);for bytes in data.chunks_exact(STRIDE){let mut scale=0_u16;for word in 0..4{let stored=u16::from_le_bytes(bytes[48+2*word..50+2*word].try_into().unwrap());scale|=(stored>>12&15)<<(4*word)}let scale=unfp16(scale);for index in 0..256{if weights.len()==count{return Ok(weights)}let block=index/16;let group=index%16/8;let high=bytes[32+block];let grid=usize::from(bytes[2*block+group])|usize::from(high>>(4*group)&7)<<8;let stored=u16::from_le_bytes(bytes[48+2*(block/4)..50+2*(block/4)].try_into().unwrap());let d=scale*f32::from(2*((stored>>(3*(block%4)))&7)+1);let delta=if high>>(3+4*group)&1!=0{-0.125}else{0.125};weights.push(f64::from(d*(f32::from(iq1_level(grid,index%8))-1.0+delta)))}}return Ok(weights)
		}
		if family == 1 && variant == 3 && bits == 3 {
			const STRIDE:usize=110;require(data.len()>=count.div_ceil(256)*STRIDE,"GGML IQ3_S weights are invalid")?;let mut weights=Vec::with_capacity(count);
			for bytes in data.chunks_exact(STRIDE) {let scale=half(bytes);for index in 0..256 {if weights.len()==count{return Ok(weights)}let block=index/32;let group=index/4;let grid=usize::from(bytes[2+group])|usize::from(bytes[66+group/8]>>(group%8)&1)<<8;let code=bytes[106+block/2]>>(block%2*4)&15;let d=scale*f32::from(1+2*code);let sign=if bytes[74+index/8]>>(index%8)&1!=0{-1.0}else{1.0};weights.push(f64::from(d*f32::from(iq3_grid(&IQ3_S,grid,index%4))*sign))}}
			return Ok(weights)
		}
		Err(RecipeError::new(format!("quantization code {} is unavailable; available GGML formats: Q4_0, Q4_1, Q5_0, Q5_1, Q8_0, Q8_1, Q2_K, Q3_K, Q4_K, Q5_K, Q6_K, Q8_K, Q4_NF, IQ1_S, IQ1_M, IQ2_XXS, IQ2_XS, IQ2_S, IQ3_XXS, IQ3_S, IQ4_NL, and IQ4_XS", self.0)))
	}
}
pub struct Qi { model: Model, pub zero: Model, pub one: Model, pub nf: Model, pub k: Qk }
pub struct Qk { model: Model, pub s: Model, pub m: Model, pub l: Model }
pub struct Iq { model: Model, pub xxs: Model, pub xs: Model, pub s: Model, pub m: Model, pub nl: Model }
impl std::ops::Deref for Qi {
	type Target = Model; fn deref(&self) -> &Model { &self.model }
}
impl std::ops::Deref for Qk {
	type Target = Model; fn deref(&self) -> &Model { &self.model }
}
impl std::ops::Deref for Iq {
	type Target = Model; fn deref(&self) -> &Model { &self.model }
}
impl Estimator {
	const fn name(&self) -> &'static str {
		self.name
	}
}
impl Operation {
	const fn name(&self) -> &'static str {
		match self {
			Self::Layer(_) => "layer",
			Self::Conv(..) => "conv",
			Self::Pool(_) => "pool",
			Self::Estimator(value) => value.name(),
			Self::Embed(..) => "embed",
			Self::Attention(_) => "attn",
			Self::Rnn(_) => "rnn",
			Self::Gru(_) => "gru",
			Self::Lstm(_) => "lstm",
			Self::Residual(_) => "residual",
			Self::Moe(..) => "moe",
			Self::Svm(_) => "svm",
			Self::Perceptron(_) => "perc",
		}
	}
}
impl Activation {
	const fn name(self) -> &'static str {
		match self {
			Self::Linear => "linear",
			Self::Cos => "cos",
			Self::Exp => "exp",
			Self::Log => "log",
			Self::Ln => "ln",
			Self::Huber => "huber",
			Self::Tan => "tan",
			Self::Relu => "relu",
			Self::Leak => "leak",
			Self::Sigmoid => "sigmoid",
			Self::Tanh => "tanh",
			Self::Selu => "selu",
			Self::Gelu => "gelu",
			Self::Silu => "silu",
			Self::Elu => "elu",
			Self::Prelu => "prelu",
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
macro_rules! activations { ($(fn $method:ident = $activation:ident;)+) => {$(impl Model { pub fn $method(&self) -> Self {
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
pub struct DeviceInfo { pub name: String, pub host: String }
#[derive(Clone)]
pub struct DeviceLink { pub from: String, pub to: String, pub latency_ms: f64, pub bytes_per_second: f64 }
#[derive(Clone)]
pub struct Topology { pub devices: Vec<DeviceInfo>, pub links: Vec<DeviceLink> }
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
		match self.0 {
			0 => "mse",
			1 => "rmse",
			2 => "huber",
			3 => "mae",
			4 => "bce",
			5 => "ce",
			6 => "focal",
			_ => unreachable!(),
		}
	}
	fn value(self, prediction: f64, target: f64, threshold: f64) -> f64 {
		let difference = prediction - target;
		let probability = logistic(prediction).clamp(f64::EPSILON, 1.0 - f64::EPSILON);
		match self.0 {
			0 | 1 => difference * difference,
			2 => {
				let absolute = difference.abs();
				if absolute <= threshold { 0.5 * difference * difference } else { threshold * (absolute - 0.5 * threshold) }
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
		Data { sources: sources.into_data_sources(), target: Vec::new(), exclusions: Vec::new(), routes: Vec::new(), normalize: false, split: 1.0, prepared: OnceLock::new() }
	}
	pub fn model(&self) -> Model {
		Model { blocks: Vec::new(), loss: mse, downstream: None, format: FP64, quantization: 0 }
	}
	pub fn devices(&self) -> Vec<String> {
		self.topology().devices.into_iter().map(|device| device.name).collect()
	}
	pub fn topology(&self) -> Topology {
		topology().unwrap_or_else(|error| panic!("{error}"))
	}
	pub const fn train(&self) -> Train {
		Train { epochs: 1, learning_rate: 0.001, log_metrics: Vec::new(), stop: None, resume: None, save: None, seed: None }
	}
	pub fn export(&self, source: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
		let source = source.as_ref();
		require(source.extension().and_then(|value| value.to_str()) == Some("rs"), "export requires a Rust source")?;
		fs::metadata(source).map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", source.display())))?;
		let found = devices()?;
		let backends = [Backend::Amd, Backend::Nvidia].into_iter().filter(|backend| found.iter().any(|gpu| gpu.backend == *backend)).collect::<Vec<_>>();
		let mut outputs = Vec::new();
		for backend in backends {
			let artifacts = match backend {
				Backend::Cpu => Vec::new(),
				Backend::Amd => vec![mapped_artifacts(option_env!("RECIPE_HSA_CODE_OBJECTS"), "hsaco")?, mapped_artifacts(option_env!("RECIPE_HSA_ASSEMBLIES"), "amd.s")?].concat(),
				Backend::Nvidia => vec![("ptx".to_owned(), option_env!("RECIPE_NV_PTX").ok_or_else(|| RecipeError::new("Nvidia artifacts were not compiled"))?)],
			};
			for (extension, compiled) in artifacts {
				let output = source.with_file_name(format!("recipe.{extension}"));
				fs::copy(compiled, &output).map_err(|error| RecipeError::new(format!("cannot export {}: {error}", output.display())))?;
				eprintln!("exported: {}", output.display());
				outputs.push(output);
			}
		}
		Ok(outputs)
	}
}
fn mapped_artifacts<'a>(mapping: Option<&'a str>, suffix: &str) -> Result<Vec<(String, &'a str)>> {
	let mapping = mapping.ok_or_else(|| RecipeError::new("AMD artifacts were not compiled"))?;
	Ok(mapping.split(';').filter_map(|value| value.split_once('=')).map(|(target, path)| (format!("{target}.{suffix}"), path)).collect())
}
fn inject_bn_stats(graph: &Graph, bn_stats: &[f64], contexts: &[Buffer]) -> Result<()> {
	if bn_stats.is_empty() {
		return Ok(());
	}
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
trait CpuFloat: Copy + Default {
	unsafe fn forward(samples: *const Self, weights: *const Self, values: *const *mut Self, contexts: *const *mut Self, descriptors: *const i32, arguments: *const Self, rows: i32, nodes: i32, timings: *mut u64, tiles: *const Tile);
	unsafe fn epoch(samples: *const Self, input_adjoint: *mut Self, targets: *const Self, weights: *mut Self, frozen: *const u8, best: *mut Self, values: *const *mut Self, contexts: *const *mut Self, adjoints: *const *mut Self, descriptors: *const i32, arguments: *const Self, metrics: *mut Self, gradient: *mut Self, moments: *mut Self, variances: *mut Self, best_loss: *mut Self, rows: i32, nodes: i32, parameters: i32, loss: i32, threshold: Self, rate: Self, beta1: Self, beta2: Self, beta1_power: Self, beta2_power: Self, epsilon: Self, decay: Self, tolerance: Self, step: i32, threads: i32, tile_m: i32, tile_n: i32, tile_k: i32, phase: i32, timings: *mut u64, tiles: *const Tile); }
impl CpuFloat for f64 {
	unsafe fn forward(samples: *const Self, weights: *const Self, values: *const *mut Self, contexts: *const *mut Self, descriptors: *const i32, arguments: *const Self, rows: i32, nodes: i32, timings: *mut u64, tiles: *const Tile) { unsafe { recipe_forward_cpu(samples, weights, values, contexts, descriptors, arguments, rows, nodes, 1, 1, 1, 1, timings, tiles) } }
	unsafe fn epoch(samples: *const Self, input_adjoint: *mut Self, targets: *const Self, weights: *mut Self, frozen: *const u8, best: *mut Self, values: *const *mut Self, contexts: *const *mut Self, adjoints: *const *mut Self, descriptors: *const i32, arguments: *const Self, metrics: *mut Self, gradient: *mut Self, moments: *mut Self, variances: *mut Self, best_loss: *mut Self, rows: i32, nodes: i32, parameters: i32, loss: i32, threshold: Self, rate: Self, beta1: Self, beta2: Self, beta1_power: Self, beta2_power: Self, epsilon: Self, decay: Self, tolerance: Self, step: i32, threads: i32, tile_m: i32, tile_n: i32, tile_k: i32, phase: i32, timings: *mut u64, tiles: *const Tile) { unsafe { recipe_epoch_cpu(samples, input_adjoint, targets, weights, frozen, best, values, contexts, adjoints, descriptors, arguments, metrics, gradient, moments, variances, best_loss, rows, nodes, parameters, loss, threshold, rate, beta1, beta2, beta1_power, beta2_power, epsilon, decay, tolerance, step, threads, tile_m, tile_n, tile_k, phase, timings, tiles) } } }
impl CpuFloat for f32 {
	unsafe fn forward(samples: *const Self, weights: *const Self, values: *const *mut Self, contexts: *const *mut Self, descriptors: *const i32, arguments: *const Self, rows: i32, nodes: i32, timings: *mut u64, tiles: *const Tile) { unsafe { recipe_forward_cpu_f32(samples, weights, values, contexts, descriptors, arguments, rows, nodes, 1, 1, 1, 1, timings, tiles) } }
	unsafe fn epoch(samples: *const Self, input_adjoint: *mut Self, targets: *const Self, weights: *mut Self, frozen: *const u8, best: *mut Self, values: *const *mut Self, contexts: *const *mut Self, adjoints: *const *mut Self, descriptors: *const i32, arguments: *const Self, metrics: *mut Self, gradient: *mut Self, moments: *mut Self, variances: *mut Self, best_loss: *mut Self, rows: i32, nodes: i32, parameters: i32, loss: i32, threshold: Self, rate: Self, beta1: Self, beta2: Self, beta1_power: Self, beta2_power: Self, epsilon: Self, decay: Self, tolerance: Self, step: i32, threads: i32, tile_m: i32, tile_n: i32, tile_k: i32, phase: i32, timings: *mut u64, tiles: *const Tile) { unsafe { recipe_epoch_cpu_f32(samples, input_adjoint, targets, weights, frozen, best, values, contexts, adjoints, descriptors, arguments, metrics, gradient, moments, variances, best_loss, rows, nodes, parameters, loss, threshold, rate, beta1, beta2, beta1_power, beta2_power, epsilon, decay, tolerance, step, threads, tile_m, tile_n, tile_k, phase, timings, tiles) } }
}
impl Recipe {
	pub fn infer(&self, path: impl AsRef<Path>, input: &[f64]) -> Vec<f64> {
		let path = path.as_ref().to_string_lossy();
		let result = devices().and_then(|devices| devices.first().ok_or_else(|| RecipeError::new("execution device is absent"))).and_then(|device| {
			bundle::run_infer(&path, input, |stored, samples| {
				let mut tape = GpuTape::new(&stored.graph, samples, &[], device, stored.precision)?;
				inject_bn_stats(&stored.graph, &stored.bn_stats, &tape.contexts)?;
				tape.forward()?;
				tape.predictions()
			})
		});
		result.unwrap_or_else(|error| panic!("{error}"))
	}
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shape { channels: usize, length: usize }
impl Shape {
	fn elements(self) -> usize {
		self.channels * self.length
	}
}
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
enum Primitive { Contraction = 0, Pool = 2, Gather = 3, Attention = 4, Scan = 5, Elementwise = 6, Route = 7, Normalize = 8, #[allow(dead_code, reason = "reserved for format-independent quantized lowering")] Quantized = 9 }
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
	fn constant(&mut self, value: f64) -> f64 {
		self.op(ScalarOpcode::Constant, value, 0.0)
	}
	fn choose(&mut self, condition: f64, yes: f64, no: f64) -> f64 {
		let one = self.constant(1.0);
		let inv = self.op(ScalarOpcode::Subtract, one, condition);
		let (a, b) = (self.op(ScalarOpcode::Multiply, condition, yes), self.op(ScalarOpcode::Multiply, inv, no));
		self.op(ScalarOpcode::Add, a, b)
	}
	fn unary(&mut self, opcode: ScalarOpcode, value: f64) -> f64 {
		self.op(opcode, value, 0.0)
	}
}
impl Node {
	fn identity(&self, index: usize) -> String {
		let prim = match self.op {
			Primitive::Contraction => "Contraction",
			Primitive::Pool => "Pool",
			Primitive::Gather => "Gather",
			Primitive::Attention => "Attention",
			Primitive::Scan => "Scan",
			Primitive::Elementwise => "Elementwise",
			Primitive::Route => "Route",
			Primitive::Normalize => "Normalize",
			Primitive::Quantized => "Quantized",
		};
		format!("block {} {}, node {} {}, input {}x{}, output {}x{}, offset={} count={}", self.block_index, self.block_kind, index, prim, self.input.channels, self.input.length, self.output.channels, self.output.length, self.offset, self.parameters)
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
			Primitive::Contraction => Some([self.output.length as f64, self.output.channels as f64, self.input.channels as f64 * self.argument[0].max(1.0)]),
			Primitive::Quantized => Some([self.output.length as f64, self.output.channels as f64, self.input.elements() as f64]),
			Primitive::Attention => Some([self.output.length as f64, self.output.channels as f64, self.input.channels as f64]),
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
		Self { nodes: Vec::new(), parameters: Vec::new(), frozen: Vec::new(), programs: Vec::new(), input: shape, output: shape, source: -1, state: TrainingState::default(), block_index: 0, block_kind: "" }
	}
}
fn compile(model: &Model, data: &Prepared, rows: usize, gpu: &'static Gpu, config: Config) -> Result<Graph> {
	compile_graph(model, data, rows, gpu, config, None)
}
fn compile_output(model: &Model, data: &Prepared, rows: usize, gpu: &'static Gpu, config: Config, output: usize) -> Result<Graph> {
	compile_graph(model, data, rows, gpu, config, Some(output))
}
fn compile_graph(model: &Model, data: &Prepared, rows: usize, gpu: &'static Gpu, config: Config, expected: Option<usize>) -> Result<Graph> {
	require(!model.blocks.is_empty(), "model must contain a block")?;
	let sequential = matches!(model.blocks[0].operation, Operation::Conv(..) | Operation::Pool(..) | Operation::Embed(..));
	let shape = if sequential { data.sequence.unwrap_or(Shape { channels: 1, length: data.features }) } else { Shape { channels: data.features, length: 1 } };
	let mut graph = Graph::new(shape);
	for (index, block) in model.blocks.iter().enumerate() {
		graph.block_index = index;
		graph.block_kind = block.operation.name();
		lower_block(&mut graph, block, model.blocks.len(), data, rows, gpu, config)?;
	}
	let mut output_profile=model.blocks.last().filter(|block|block.profile).map(|block|IntegerFormat(block.quantization));
	if let Some(expected) = expected {
		require(graph.output.elements() == expected, "model output width does not match .out()")?;
	} else if graph.output.elements() != 1 {
		let length = graph.output.length;
		lower_conv(&mut graph, 1, length)?;
		if model.quantization!=0{graph.nodes.last_mut().unwrap().argument[8]=f64::from(model.quantization)}output_profile=IntegerFormat(model.quantization).selection().map(|_|IntegerFormat(model.quantization));
	}
	if let Some(format)=output_profile&&let Some(node)=graph.nodes.iter_mut().rev().find(|node|node.parameters!=0&&node.block_index+1==model.blocks.len()){node.argument[8]=f64::from(format.tensor(0,false,true))}
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
struct Field { source: i32, stride: usize, index: usize }
fn field(fields: &[(String, Field)], name: &str) -> Result<Field> {
	fields.iter().find(|value| value.0 == name).map(|value| value.1).ok_or_else(|| RecipeError::new(format!("RAT value {name:?} is not yet available")))
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
fn lower_block(graph: &mut Graph, block: &Block, total:usize, data: &Prepared, rows: usize, gpu: &'static Gpu, config: Config) -> Result<()> {
	let skip = graph.source;
	let first = graph.nodes.len();
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
		push_node(graph, Primitive::Normalize, graph.output, 0, arguments(normalization as u8 as f64, epsilon), -2)?;
	}
	if block.quantization != 0 {
		let more=graph.block_index<total/8||graph.block_index>=7*total/8||(graph.block_index-total/8)%3==2;let mut parameter=0;
		for node in &mut graph.nodes[first..] {
			if node.parameters != 0 {
				let role=if block.operation.name()=="attn"{parameter}else{0};node.argument[8]=f64::from(if block.profile{IntegerFormat(block.quantization).tensor(role,more,false)}else{block.quantization});parameter+=1
			}
		}
	}
	let elements = checked_mul(rows, graph.output.elements(), "node batch")?;
	narrow(elements, "GPU node batch")?;
	Ok(())
}
fn push_node(graph: &mut Graph, op: Primitive, output: Shape, parameters: usize, argument: [f64; 9], second: i32) -> Result<()> {
	let (source, offset) = (graph.source, graph.parameters.len());
	graph.parameters.resize(checked_add(offset, parameters, "model parameters")?, 0.0);
	graph.frozen.resize(graph.parameters.len(), 0);
	graph.nodes.push(Node { op, source, second, input: graph.output, output, offset, parameters, argument, program_offset: 0, program_count: 0, block_index: graph.block_index, block_kind: graph.block_kind });
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
					let alpha = constant(&mut program, config.activation[usize::from(activation == Activation::Selu) + 2]);
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
	let (parameters, output) = (checked_add(checked_mul(graph.output.channels, channels, "projection matrix")?, channels, "projection bias")?, Shape { channels, length: graph.output.length });
	push_node(graph, Primitive::Contraction, output, parameters, [0.0; 9], -2)
}
fn lower_conv(graph: &mut Graph, filters: usize, kernel: usize) -> Result<()> {
	require(filters != 0 && kernel != 0, "convolution dimensions must be positive")?;
	require(kernel <= graph.output.length, "convolution kernel exceeds sequence length")?;
	let parameters = checked_add(checked_mul(filters, checked_mul(graph.output.channels, kernel, "convolution window")?, "conv matrix")?, filters, "conv bias")?;
	let output = Shape { channels: filters, length: graph.output.length - kernel + 1 };
	push_node(graph, Primitive::Contraction, output, parameters, arguments(kernel as f64, 0.0), -2)
}
fn output_bias_offset(graph: &Graph) -> Option<usize> {
	graph.nodes.iter().rev().find(|node| node.op == Primitive::Contraction).map(|node| node.offset + node.parameters - node.output.channels)
}
fn lower_pool(graph: &mut Graph, size: usize) -> Result<()> {
	require(size != 0, "pool window must be positive")?;
	let output = Shape { channels: graph.output.channels, length: graph.output.length.div_ceil(size) };
	push_node(graph, Primitive::Pool, output, 0, arguments(size as f64, 0.0), -2)
}
fn lower_gather(graph: &mut Graph, dimensions: usize, vocabulary: usize) -> Result<()> {
	require(dimensions != 0 && vocabulary != 0, "embedding dimensions must be positive")?;
	let (parameters, output) = (checked_mul(dimensions, vocabulary, "embedding table")?, Shape { channels: dimensions, length: graph.output.elements() });
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
	push_node(graph, Primitive::Attention, input, 0, [heads as f64, heads as f64, width as f64, 0.0, 0.0, (width as f64).sqrt(), 0.0, 0.0, 0.0], -2)?;
	lower_project(graph, input.channels)
}
const ACTIVATIONS: [Activation; 16] = [Activation::Linear, Activation::Cos, Activation::Exp, Activation::Log, Activation::Ln, Activation::Huber, Activation::Tan, Activation::Relu, Activation::Leak, Activation::Sigmoid, Activation::Tanh, Activation::Selu, Activation::Gelu, Activation::Silu, Activation::Elu, Activation::Prelu];
fn reset(graph: &mut Graph, source: i32, shape: Shape) {
	graph.source = source;
	graph.output = shape;
}
fn program(graph: &mut Graph, first: i32, second: i32, shape: Shape, initial: &[f64], program: ScalarProgram) -> Result<i32> {
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
fn select(graph: &mut Graph, branches: &[i32], scores: &[i32], shape: Shape, top_k: usize, config: Config) -> Result<()> {
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
	let (input, state) = (checked_mul(graph.output.channels, channels, "scan input matrix")?, checked_mul(channels, channels, "scan state matrix")?);
	let stride = checked_add(checked_add(input, state, "scan gate")?, channels, "scan bias")?;
	let output = Shape { channels, length: graph.output.length };
	push_node(graph, Primitive::Scan, output, checked_mul(gates, stride, "scan parameters")?, arguments(gates as f64, 0.0), -2)
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
	require(graph.output.channels == shape.channels && graph.output.length == shape.length, "residual shape mismatch")?;
	let mut program = ScalarProgram(Vec::new());
	program.op(ScalarOpcode::Add, -1.0, -2.0);
	push_program(graph, skip, &[], program)
}
fn lower_estimator(graph: &mut Graph, estimator: &Estimator, data: &Prepared, rows: usize, gpu: &'static Gpu, config: Config) -> Result<()> {
	let input = graph.output;
	let inputs = graph_inputs(graph, &data.samples, &data.targets, rows, gpu, config.precision)?;
	let mut samples = inputs.clone();
	samples.extend_from_slice(&inputs);
	let mut targets = data.targets[..rows].to_vec();
	targets.extend_from_within(..);
	let paired = Prepared { samples, targets, rows: checked_mul(rows, 2, "paired estimator rows")?, features: input.elements(), schema: String::new(), sequence: None, norm_mean: Vec::new(), norm_scale: Vec::new(), identities: Vec::new() };
	let teacher = {
		require(paired.rows > rows, "estimator split must retain test rows")?;
		let predict = estimator.fit(&paired, rows, config, true)?;
		paired.samples[rows * input.elements()..].chunks_exact(input.elements()).enumerate().map(|(row, query)| predict(row, query)).collect::<Vec<_>>()
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
				graph.parameters[node.offset + gate * stride + input_matrix + state_matrix..node.offset + (gate + 1) * stride].fill(0.0);
			}
			if node.argument[0] as usize == 4 {
				graph.parameters[node.offset + stride + input_matrix + state_matrix..node.offset + stride * 2].fill(1.0);
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
		let tile = Self { m: dimension(values[0], minimum.m, maximum.m)?, n: dimension(values[1], minimum.n, maximum.n)?, k: dimension(values[2], minimum.k, maximum.k)? };
		values[..3].copy_from_slice(&[f64::from(tile.m), f64::from(tile.n), f64::from(tile.k)]);
		Ok(tile)
	}
}
#[derive(Clone, Copy)]
struct Config {
	kmeans_iterations: usize,
	quantization_block: usize,
	surrogate_epochs: usize,
	surrogate_width: usize,
	surrogate_rate: f64,
	initial: f64,
	beta1: f64,
	beta2: f64,
	epsilon: f64,
	decay: f64,
	rat_batch: usize,
	vram_unit: usize,
	random_seed: usize,
	activation: [f64; 8],
	precision: FloatFormat,
}
impl Config {
	fn load() -> Result<Self> {
		Ok(Self { kmeans_iterations: natural("kmeans iterations", env!("RECIPE_KMEANS_ITERATIONS"))?, quantization_block: natural("quantization block weights", env!("RECIPE_QUANTIZATION_BLOCK_WEIGHTS"))?, surrogate_epochs: natural("surrogate epochs", env!("RECIPE_SURROGATE_EPOCHS"))?, surrogate_width: natural("surrogate width", env!("RECIPE_SURROGATE_WIDTH"))?, surrogate_rate: number("surrogate rate", env!("RECIPE_SURROGATE_RATE"))?, rat_batch: natural("RAT batch rows", env!("RECIPE_RAT_BATCH_ROWS"))?, vram_unit: natural("tile VRAM unit bytes", env!("RECIPE_TILE_VRAM_UNIT_BYTES"))?, random_seed: natural("random seed", env!("RECIPE_RANDOM_SEED"))?, initial: number("initial weight", env!("RECIPE_TRAIN_INITIAL_WEIGHT"))?, beta1: number("AdamW beta1", env!("RECIPE_ADAMW_BETA1"))?, beta2: number("AdamW beta2", env!("RECIPE_ADAMW_BETA2"))?, epsilon: number("AdamW epsilon", env!("RECIPE_ADAMW_EPSILON"))?, decay: number("AdamW weight decay", env!("RECIPE_ADAMW_WEIGHT_DECAY"))?, activation: [number("leak slope", env!("RECIPE_LEAK_SLOPE"))?, number("PReLU slope", env!("RECIPE_PRELU_SLOPE"))?, number("ELU alpha", env!("RECIPE_ELU_ALPHA"))?, number("SELU alpha", env!("RECIPE_SELU_ALPHA"))?, number("SELU scale", env!("RECIPE_SELU_SCALE"))?, number("GELU scale", env!("RECIPE_GELU_SCALE"))?, number("GELU cubic", env!("RECIPE_GELU_CUBIC"))?, number("Huber threshold", env!("RECIPE_HUBER_THRESHOLD"))?], precision: FP64 })
	}
}
fn number(name: &str, text: &str) -> Result<f64> {
	let value = text.parse::<f64>().map_err(|error| RecipeError::new(format!("invalid {name}: {error}")))?;
	(value.is_finite() && value > 0.0).then_some(value).ok_or_else(|| RecipeError::new(format!("{name} must be finite and positive")))
}
fn natural(name: &str, text: &str) -> Result<usize> {
	let value = text.parse::<usize>().map_err(|error| RecipeError::new(format!("invalid {name}: {error}")))?;
	require(value != 0, format!("{name} must be positive")).map(|_| value)
}
fn multi_device() -> bool {
	env!("RECIPE_MULTI_DEVICE") == "true"
}
fn stored_graph(graph: &Graph, data: &Data, scale: Option<TargetScale>, precision: FloatFormat) -> bundle::StoredGraph {
	let inputs = (0..graph.input.elements()).map(|index| format!("input{index}")).collect();
	let output = data.target.first().cloned().unwrap_or_else(|| "target".to_owned());
	let (norm_mean, norm_scale) = match data.prepared.get() {
		Some(Ok(prepared)) => (prepared.norm_mean.clone(), prepared.norm_scale.clone()),
		_ => (Vec::new(), Vec::new()),
	};
	let (target_min, target_span) = scale.map_or((0.0, 0.0), |s| (s.minimum, s.span));
	bundle::StoredGraph { graph: graph.clone(), precision, inputs, outputs: vec![output], norm_mean, norm_scale, target_min, target_span, bn_stats: Vec::new() }
}
struct GpuTape {
	gpu: &'static Gpu,
	precision: FloatFormat,
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
	fn new(graph: &Graph, samples: &[f64], targets: &[f64], gpu: &'static Gpu, precision: FloatFormat) -> Result<Self> {
		let inputs = graph.input.elements();
		require(inputs != 0 && !samples.is_empty() && samples.len() % inputs == 0, "model input batch is invalid")?;
		let rows = samples.len() / inputs;
		let training = !targets.is_empty();
		require(targets.is_empty() || targets.len() == rows || targets.len() == rows * graph.output.elements(), "target batch is invalid")?;
		let tile = gpu.prior_tile(graph, precision)?;
		let element = precision.bytes();
		let (mut descriptors, mut arguments, mut values, mut contexts, mut adjoints) = (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
		let program_base = checked_mul(graph.nodes.len(), 9, "node arguments")?;
		for (index, node) in graph.nodes.iter().enumerate() {
			descriptors.extend(node_descriptor(node, program_base)?);
			arguments.extend(node.argument);
			let elements = graph_rows_buffer(node.output, rows, element)?;
			let id = || node.identity(index);
			values.push(Buffer::new(gpu, elements).map_err(|e| RecipeError::new(format!("{e}, {}", id())))?);
			let adjoint_bytes = if training { elements } else { element };
			adjoints.push(Buffer::upload(gpu, &vec![0_u8; adjoint_bytes]).map_err(|e| RecipeError::new(format!("{e}, {}", id())))?);
			let context = Buffer::new(gpu, node_context(node, rows, element)?).map_err(|e| RecipeError::new(format!("{e}, {}", id())))?;
			if node.op == Primitive::Attention && node.argument[4] == 1.0 {
				context.write_float(0, &[0.0], precision)?;
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
		Ok(Self { gpu, precision, value_pointers: Buffer::upload(gpu, &addresses(&values))?, context_pointers: Buffer::upload(gpu, &addresses(&contexts))?, adjoint_pointers: Buffer::upload(gpu, &addresses(&adjoints))?, descriptors: Buffer::upload(gpu, &descriptors)?, arguments: Buffer::upload_float(gpu, &arguments, precision)?, samples: Buffer::upload_float(gpu, samples, precision)?, timings: Buffer::upload(gpu, &vec![0_u64; graph.nodes.len().max(1)])?, tiles: Buffer::upload(gpu, &vec![tile; graph.nodes.len().max(1)])?, input_adjoint: Buffer::upload_float(gpu, &vec![0.0; if training { samples.len() } else { 1 }], precision)?, targets: Buffer::upload_float(gpu, &target_values, precision)?, weights: Buffer::upload_float(gpu, &graph.parameters, precision)?, frozen: Buffer::upload(gpu, if training && !graph.frozen.is_empty() { &graph.frozen } else { &[1] })?, best: Buffer::upload_float(gpu, if training { resumed(&graph.state.best, &graph.parameters) } else { &zeros }, precision)?, moments: Buffer::upload_float(gpu, if training { resumed(&graph.state.moments, &zeros) } else { &zeros }, precision)?, variances: Buffer::upload_float(gpu, if training { resumed(&graph.state.variances, &zeros) } else { &zeros }, precision)?, gradient: Buffer::upload_float(gpu, &zeros, precision)?, metrics: Buffer::upload_float(gpu, &[0.0, 0.0, 0.0], precision)?, best_loss: Buffer::upload_float(gpu, if training { resumed(&graph.state.best_loss, &cold_loss) } else { &zeros }, precision)?, rows: narrow(rows, "GPU rows")? as u32, nodes: narrow(graph.nodes.len(), "GPU nodes")? as u32, parameters: narrow(graph.parameters.len(), "GPU parameters")? as u32, threads: 0, step: narrow(graph.state.epoch, "optimizer epoch")? as u32, input: graph.input.elements(), output: graph.output.elements(), values, contexts: contexts, _adjoints: adjoints, capacity: rows, tile })
	}
	fn forward(&mut self) -> Result<()> {
		let d = self.gpu.kernels(self.precision)?.0;
		self.threads = d.geometry.threads(u32::MAX)?;
		let mut a = self.forward_arguments();
		self.gpu.launch(d, &mut a, self.threads, self.tile)
	}
	fn forward_arguments(&mut self) -> [*mut c_void; 14] {
		ptrs![self.samples.pointer, self.weights.pointer, self.value_pointers.pointer, self.context_pointers.pointer, self.descriptors.pointer, self.arguments.pointer, self.rows, self.nodes, self.threads, self.tile.m, self.tile.n, self.tile.k, self.timings.pointer, self.tiles.pointer]
	}
	fn timings(&self) -> Result<Vec<u64>> {
		self.timings.download(self.nodes as usize)
	}
	fn predictions(&self) -> Result<Vec<f64>> {
		self.values.last().ok_or_else(|| RecipeError::new("GPU tape is empty"))?.download_float(self.rows as usize * self.output, self.precision)
	}
	fn launch_epoch(&mut self, rate: f64, loss: LossFunction, tolerance: f64, config: Config, direct: bool, phase: EpochPhase) -> Result<()> {
		let mut loss = if direct { 7 } else { loss.0 as u32 };
		let mut huber_threshold = config.activation[7];
		require(self.step != 0, "optimizer epoch is absent")?;
		let (mut step, mut rate, mut beta1, mut beta2, mut epsilon, mut decay, mut tolerance) = (self.step, rate, config.beta1, config.beta2, config.epsilon, config.decay, tolerance);
		let (mut beta1_power, mut beta2_power) = (config.beta1.powi(self.step as i32), config.beta2.powi(self.step as i32));
		let mut phase = phase as u32;
		let mut call = ptrs![self.samples.pointer, self.input_adjoint.pointer, self.targets.pointer, self.weights.pointer, self.frozen.pointer, self.best.pointer, self.value_pointers.pointer, self.context_pointers.pointer, self.adjoint_pointers.pointer, self.descriptors.pointer, self.arguments.pointer, self.metrics.pointer, self.gradient.pointer, self.moments.pointer, self.variances.pointer, self.best_loss.pointer, self.rows, self.nodes, self.parameters, loss, huber_threshold, rate, beta1, beta2, beta1_power, beta2_power, epsilon, decay, tolerance, step, self.threads, self.tile.m, self.tile.n, self.tile.k, phase, self.timings.pointer, self.tiles.pointer];
		let mut narrow = [huber_threshold as f32, rate as f32, beta1 as f32, beta2 as f32, beta1_power as f32, beta2_power as f32, epsilon as f32, decay as f32, tolerance as f32];
		let mut half = narrow.map(fp16);
		if self.precision.native() == Some(FP16) { for (slot, value) in call[20..29].iter_mut().zip(&mut half) { *slot = value as *mut u16 as Ptr } }
		if self.precision.native() == Some(FP32) {
			for (slot, value) in call[20..29].iter_mut().zip(&mut narrow) {
				*slot = value as *mut f32 as Ptr;
			}
		}
		let dispatch = self.gpu.kernels(self.precision)?.1;
		self.threads = dispatch.geometry.threads(self.rows)?;
		self.gpu.launch(dispatch, &mut call, self.threads, self.tile)
	}
	fn epoch(&mut self, rate: f64, loss: LossFunction, tolerance: f64, config: Config, direct: bool) -> Result<(f64, bool)> {
		self.launch_epoch(rate, loss, tolerance, config, direct, EpochPhase::Full)?;
		let metrics = self.metrics.download_float(3, self.precision)?;
		Ok((metrics[0], metrics[1] != 0.0))
	}
	fn gradient(&mut self, rate: f64, loss: LossFunction, tolerance: f64, config: Config, direct: bool) -> Result<()> {
		self.launch_epoch(rate, loss, tolerance, config, direct, EpochPhase::Gradient)
	}
	fn update(&mut self, rate: f64, loss: LossFunction, tolerance: f64, config: Config, direct: bool) -> Result<()> {
		self.launch_epoch(rate, loss, tolerance, config, direct, EpochPhase::Optimizer)
	}
	fn advance(&mut self) -> Result<()> {
		self.step = self.step.checked_add(1).ok_or_else(|| RecipeError::new("optimizer epoch overflows"))?;
		Ok(())
	}
	fn write_samples(&self, values: &[f64]) -> Result<()> {
		require(values.len() == self.capacity * self.input, "RAT input batch has the wrong shape")?;
		self.samples.write_float(0, values, self.precision)
	}
	fn write_targets(&self, values: &[f64]) -> Result<()> {
		require(values.len() == self.capacity * self.output, "RAT target batch has the wrong shape")?;
		self.targets.write_float(0, values, self.precision)
	}
	fn trainable(&self, graph: &Graph, range: (usize, usize)) -> Result<()> {
		require(graph.frozen.len() == self.parameters as usize && range.1 <= graph.frozen.len(), "RAT model range is invalid")?;
		let frozen = graph.frozen.iter().enumerate().map(|(index, value)| u8::from(*value != 0 || index < range.0 || index >= range.1)).collect::<Vec<_>>();
		self.frozen.write(0, &frozen)
	}
	fn node_values(&self, node: usize, width: usize) -> Result<Vec<f64>> {
		require(node < self.values.len(), "RAT proposal node is absent")?;
		self.values[node].download_float(self.capacity * width, self.precision)
	}
	fn weights(&self) -> Result<Vec<f64>> {
		self.weights.download_float(self.parameters as usize, self.precision)
	}
	fn capture(&self, graph: &mut Graph) -> Result<()> {
		graph.parameters = self.weights()?;
		graph.state.moments = self.moments.download_float(self.parameters as usize, self.precision)?;
		graph.state.variances = self.variances.download_float(self.parameters as usize, self.precision)?;
		graph.state.epoch = self.step as usize;
		Ok(())
	}
	fn gradient_values(&self) -> Result<Vec<f64>> {
		self.gradient.download_float(self.parameters as usize, self.precision)
	}
	fn write_gradient(&self, values: &[f64]) -> Result<()> {
		self.gradient.write_float(0, values, self.precision)
	}
	fn optimizer_state(&self) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
		Ok((self.weights()?, self.moments.download_float(self.parameters as usize, self.precision)?, self.variances.download_float(self.parameters as usize, self.precision)?))
	}
	fn write_tiles(&mut self, values: &[Tile]) -> Result<()> {
		require(values.len() == self.nodes as usize, "GPU tile count does not match graph nodes")?;
		self.tile = values.iter().fold(Tile { m: 1, n: 1, k: 1 }, |prior, tile| Tile { m: prior.m.max(tile.m), n: prior.n.max(tile.n), k: prior.k.max(tile.k) });
		self.tiles.write(0, values)
	}
	fn fill_tile(&mut self, tile: Tile) -> Result<()> {
		self.write_tiles(&std::iter::repeat_n(tile, self.nodes as usize).collect::<Vec<_>>())
	}
	fn write_weights(&self, values: &[f64]) -> Result<()> {
		self.weights.write_float(0, values, self.precision)
	}
}
fn tape_bytes(graph: &Graph, rows: usize, targets: bool, precision: FloatFormat) -> Result<usize> {
	let element = precision.bytes();
	let mut bytes = 0_usize;
	let mut add = |value| -> Result<()> {
		bytes = checked_add(bytes, value, "GPU tape bytes")?;
		Ok(())
	};
	for node in &graph.nodes {
		let values = graph_rows_buffer(node.output, rows, element)?;
		add(if targets { checked_mul(2, values, "GPU values and adjoints")? } else { values })?;
		add(node_context(node, rows, element)?)?;
	}
	let pointers = checked_mul(3 * graph.nodes.len(), size_of::<u64>(), "GPU tape pointers")?;
	add(pointers)?;
	add(checked_mul(graph.nodes.len() * 11, size_of::<i32>(), "GPU descriptors")?)?;
	add(checked_mul(graph.nodes.len() * 9 + graph.programs.len(), element, "GPU arguments")?)?;
	let samples = graph_rows_buffer(graph.input, rows, element)?;
	add(if targets { checked_mul(2, samples, "GPU samples and input adjoints")? } else { samples })?;
	add(checked_mul(graph.nodes.len().max(1), size_of::<u64>() + size_of::<Tile>(), "GPU node metadata")?)?;
	add(if targets { graph_rows_buffer(graph.output, rows, element)? } else { element })?;
	let parameters = checked_mul(graph.parameters.len().max(1), element, "GPU parameter bytes")?;
	add(checked_mul(if targets { 5 } else { 1 }, parameters, "GPU parameter bytes")?)?;
	add(if targets { graph.frozen.len().max(1) } else { 1 })?;
	add(7 * element)?;
	Ok(bytes)
}
fn tape_row_limit(graph: &Graph, memory: u64, rows: usize, targets: bool, precision: FloatFormat) -> Result<usize> {
	let memory = usize::try_from(memory).unwrap_or(usize::MAX);
	let (mut low, mut high) = (0, rows);
	while low < high {
		let middle = low + (high - low).div_ceil(2);
		if tape_bytes(graph, middle, targets, precision)? <= memory {
			low = middle;
		} else {
			high = middle - 1;
		}
	}
	Ok(low)
}
struct Placement { tape: GpuTape, rows: Vec<usize>, share: f64 }
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
	fn new(graph: &Graph, samples: &[f64], targets: &[f64], gpus: &[&'static Gpu], precision: FloatFormat) -> Result<Self> {
		require(!gpus.is_empty(), "execution requires a GPU")?;
		let input = graph.input.elements();
		require(input != 0 && !samples.is_empty() && samples.len() % input == 0, "model input batch is invalid")?;
		let (capacity, output) = (samples.len() / input, graph.output.elements());
		require(targets.is_empty() || targets.len() == capacity * output, "target batch is invalid")?;
		let limits = gpus.iter().map(|gpu| tape_row_limit(graph, gpu.memory, capacity, !targets.is_empty(), precision)).collect::<Result<Vec<_>>>()?;
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
				let index = viable.iter().enumerate().filter(|(index, (_, limit))| rows[*index].len() < *limit).max_by_key(|(index, (_, limit))| limit.saturating_mul(row + 1).saturating_sub(rows[*index].len().saturating_mul(total))).map(|(index, _)| index).ok_or_else(|| RecipeError::new("GPU row placement is exhausted"))?;
				rows[index].push(row);
			}
		}
		let viable_count = viable.len();
		let mut shards = Vec::new();
		for ((&gpu, _), rows) in viable.into_iter().zip(rows) {
			let share = if replicated { 1.0 / viable_count as f64 } else { rows.len() as f64 / capacity as f64 };
			let packed_targets = if targets.is_empty() { Vec::new() } else { Self::pack(targets, output, &rows) };
			let tape = GpuTape::new(graph, &Self::pack(samples, input, &rows), &packed_targets, gpu, precision)?;
			shards.push(Placement { tape, rows, share });
		}
		let best = if graph.state.best.is_empty() { graph.parameters.clone() } else { graph.state.best.clone() };
		require(best.len() == graph.parameters.len(), "saved best model has the wrong shape")?;
		let best_loss = if graph.state.best_loss.is_empty() { [f64::INFINITY, f64::NAN, f64::NAN, f64::INFINITY] } else { graph.state.best_loss.as_slice().try_into().map_err(|_| RecipeError::new("saved loss state is invalid"))? };
		let step = narrow(graph.state.epoch, "optimizer epoch")? as u32;
		let saved = |values: &Vec<f64>| if values.is_empty() { [0.0].repeat(best.len()) } else { values.clone() };
		let (moments, variances) = (saved(&graph.state.moments), saved(&graph.state.variances));
		Ok(Self { shards, targets: targets.to_vec(), best, best_moments: moments, best_variances: variances, best_epoch: step, best_loss, replicated, input, output, capacity, step })
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
			workers.into_iter().map(|worker| worker.join().map_err(|_| RecipeError::new("GPU dispatch panicked"))?).collect::<Result<Vec<_>>>()
		})
	}
	fn forward(&mut self) -> Result<()> {
		self.map(GpuTape::forward).map(|_| ())
	}
	fn collect(&self, width: usize, values: impl Fn(&GpuTape) -> Result<Vec<f64>>) -> Result<Vec<f64>> {
		if self.replicated {
			return values(&self.shards[0].tape);
		}
		let mut result = vec![0.0; self.capacity * width];
		for shard in &self.shards {
			let local = values(&shard.tape)?;
			for (index, &row) in shard.rows.iter().enumerate() {
				result[row * width..(row + 1) * width].copy_from_slice(&local[index * width..(index + 1) * width]);
			}
		}
		Ok(result)
	}
	fn predictions(&self) -> Result<Vec<f64>> {
		self.collect(self.output, GpuTape::predictions)
	}
	fn advance(&mut self) -> Result<()> {
		self.map(|tape| tape.advance())?;
		self.step = self.shards[0].tape.step;
		Ok(())
	}
	fn observe(&mut self, loss: f64, tolerance: f64) -> Result<bool> {
		let (old_best, last, trail) = (self.best_loss[0], self.best_loss[1], self.best_loss[2]);
		let better = loss < old_best;
		if better {
			(self.best, self.best_moments, self.best_variances) = self.shards[0].tape.optimizer_state()?;
			self.best_epoch = self.shards[0].tape.step.saturating_sub(1);
		}
		let next_trail = if last.is_finite() && !trail.is_finite() && loss > last { last } else { trail };
		let trigger = next_trail.is_finite() && loss > last * (1.0 + tolerance) && loss < next_trail && tolerance > 0.0;
		self.best_loss[0] = if better { loss } else { old_best };
		self.best_loss[1] = loss;
		self.best_loss[2] = next_trail;
		if trigger {
			self.best_loss[3] = self.best_loss[0];
		}
		Ok(trigger)
	}
	fn epoch(&mut self, rate: f64, loss: LossFunction, tolerance: f64, config: Config, direct: bool) -> Result<(f64, bool)> {
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
		let objective = if direct { 0.0 } else { model_loss(&self.predictions()?, &self.targets, loss, config.activation[7]) };
		let saved = if direct { false } else { self.observe(objective, tolerance)? };
		self.map(|tape| {
			tape.write_gradient(&gradient)?;
			tape.update(rate, loss, tolerance, config, direct)
		})?;
		Ok((objective, saved))
	}
	fn write_samples(&self, values: &[f64]) -> Result<()> {
		require(values.len() == self.capacity * self.input, "RAT input batch has the wrong shape")?;
		for shard in &self.shards {
			shard.tape.write_samples(&Self::pack(values, self.input, &shard.rows))?
		}
		Ok(())
	}
	fn write_targets(&mut self, values: &[f64]) -> Result<()> {
		require(values.len() == self.capacity * self.output, "RAT target batch has the wrong shape")?;
		self.targets.copy_from_slice(values);
		for shard in &self.shards {
			shard.tape.write_targets(&Self::pack(values, self.output, &shard.rows))?
		}
		Ok(())
	}
	fn trainable(&self, graph: &Graph, range: (usize, usize)) -> Result<()> {
		for shard in &self.shards {
			shard.tape.trainable(graph, range)?
		}
		Ok(())
	}
	fn node_values(&self, node: usize, width: usize) -> Result<Vec<f64>> {
		self.collect(width, |tape| tape.node_values(node, width))
	}
	fn placement(&self, row: usize) -> Result<usize> {
		require(row < self.capacity, "GPU row is out of range")?;
		Ok(row % self.shards.len())
	}
	fn proposal_limit(&self, row: usize, values: [f64; 3]) -> Result<Tile> {
		let tape = &self.shards[self.placement(row)?].tape;
		tape.gpu.proposal_limit(values, tape.precision)
	}
	fn set_tile(&mut self, row: usize, tile: Tile) -> Result<()> {
		let p = self.placement(row)?;
		self.shards[p].tape.fill_tile(tile)
	}
	fn weights(&self) -> Result<Vec<f64>> {
		self.shards[0].tape.weights()
	}
	fn restore_best(&mut self) -> Result<()> {
		for shard in &self.shards {
			shard.tape.write_weights(&self.best)?
		}
		Ok(())
	}
	fn capture(&self, graph: &mut Graph, best: bool) -> Result<()> {
		self.shards[0].tape.capture(graph)?;
		if best {
			graph.parameters = self.best.clone();
			(graph.state.moments, graph.state.variances) = (self.best_moments.clone(), self.best_variances.clone());
			graph.state.epoch = self.best_epoch as usize;
		}
		graph.state.best = self.best.clone();
		graph.state.best_loss = self.best_loss.to_vec();
		Ok(())
	}
	fn tile(&self) -> Tile {
		self.shards[0].tape.tile
	}
}
fn checkpoint(path: &str, schema: &str, stored: &mut bundle::StoredGraph, tape: &DeviceTape, best: bool) -> Result<()> {
	if let Ok((_, saved)) = bundle::load(path) {
		if saved.first().and_then(|g| g.graph.state.best_loss.first().copied()).is_some_and(|v| v <= tape.best_loss[0]) {
			return Ok(eprintln!("kept: {path}"));
		}
	}
	tape.capture(&mut stored.graph, best)?;
	bundle::save(path, schema, std::slice::from_ref(stored))
}
fn node_descriptor(node: &Node, program_base: usize) -> Result<[i32; 11]> {
	let program_offset = if node.program_count == 0 { 0 } else { checked_add(program_base, node.program_offset, "scalar program offset")? };
	Ok([node.op as i32, node.source, node.second, narrow(node.input.channels, "input channels")?, narrow(node.input.length, "input length")?, narrow(node.output.channels, "output channels")?, narrow(node.output.length, "output length")?, narrow(node.offset, "weight offset")?, narrow(node.parameters, "parameter count")?, narrow(program_offset, "program offset")?, narrow(node.program_count, "scalar instruction count")?])
}
fn graph_rows_buffer(shape: Shape, rows: usize, element: usize) -> Result<usize> {
	checked_mul(checked_mul(rows, shape.elements(), "node elements")?, element, "node bytes")
}
fn node_context(node: &Node, rows: usize, element: usize) -> Result<usize> {
	if node.op == Primitive::Attention && node.argument[4] == 1.0 {
		let values = checked_mul(node.argument[3] as usize, 2 * node.argument[1] as usize * node.argument[2] as usize, "attention cache")?;
		return checked_add(element, checked_mul(values, size_of::<u16>(), "FP16 attention cache")?, "attention context");
	}
	let elements = match node.op {
		Primitive::Elementwise => checked_mul(2 * node.program_count, checked_mul(rows, node.output.elements(), "program batch")?, "program")?,
		Primitive::Scan => {
			let (state_count, gates) = (checked_mul(rows, node.output.elements(), "scan batch")?, node.argument[0] as usize);
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
	checked_mul(elements.max(1), element, "context bytes")
}
fn narrow(value: usize, role: &str) -> Result<i32> {
	i32::try_from(value).map_err(|_| RecipeError::new(format!("{role} exceeds i32")))
}
struct Buffer { runtime: &'static Gpu, pointer: u64, bytes: usize }
impl Buffer {
	fn new(runtime: &'static Gpu, bytes: usize) -> Result<Self> {
		Ok(Self { runtime, pointer: runtime.allocate(bytes)?, bytes })
	}
	fn upload<T>(runtime: &'static Gpu, values: &[T]) -> Result<Self> {
		let buffer = Self::new(runtime, size_of_val(values))?;
		runtime.upload(buffer.pointer, values.as_ptr().cast(), size_of_val(values))?;
		Ok(buffer)
	}
	fn upload_float(runtime: &'static Gpu, values: &[f64], precision: FloatFormat) -> Result<Self> {
		if precision.native() == Some(FP16) { return Self::upload(runtime, &values.iter().map(|value| fp16(*value as f32)).collect::<Vec<_>>()) }
		if precision.native() == Some(FP32) {
			return Self::upload(runtime, &values.iter().map(|value| *value as f32).collect::<Vec<_>>());
		}
		Self::upload(runtime, values)
	}
	fn write<T>(&self, offset: usize, values: &[T]) -> Result<()> {
		let start = checked_mul(offset, size_of::<T>(), "GPU write offset")?;
		require(checked_add(start, size_of_val(values), "GPU write")? <= self.bytes, "GPU write exceeds buffer")?;
		self.runtime.upload(self.pointer + start as u64, values.as_ptr().cast(), size_of_val(values))
	}
	fn write_float(&self, offset: usize, values: &[f64], precision: FloatFormat) -> Result<()> {
		if precision.native() == Some(FP16) { return self.write(offset, &values.iter().map(|value| fp16(*value as f32)).collect::<Vec<_>>()) }
		if precision.native() == Some(FP32) {
			return self.write(offset, &values.iter().map(|value| *value as f32).collect::<Vec<_>>());
		}
		self.write(offset, values)
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
	fn download_float(&self, count: usize, precision: FloatFormat) -> Result<Vec<f64>> {
		if precision.native() == Some(FP16) { return Ok(self.download::<u16>(count)?.into_iter().map(|value| f64::from(unfp16(value))).collect()) }
		if precision.native() == Some(FP32) {
			return Ok(self.download::<f32>(count)?.into_iter().map(f64::from).collect());
		}
		self.download(count)
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
	element: u8,
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
	const fn remote(object: u64, shared: u32, element: u8, layout: &'static [u8]) -> Self {
		Self {
			object,
			shared,
			element,
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
const EPOCH_F32_ARGS: &[u8] = b"8888888888888888444444444444444444488";
const EPOCH_F16_ARGS: &[u8] = b"8888888888888888444422222222244444488";
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
	const fn cuda(object: usize, shared: u32, element: u8, _layout: &'static [u8]) -> Self {
		Self {
			object: object as u64,
			shared,
			element,
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
	Cpu,
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
	kernels: [Option<(Dispatch, Dispatch)>; 3],
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
#[cfg(nvidia)]
type NvQuery = unsafe extern "C" fn(*mut i32, i32, i32) -> i32;
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
unsafe fn cpu_argument<T: Copy>(arguments: &[Ptr], index: usize) -> T { unsafe { *arguments[index].cast::<T>() } }
unsafe fn cpu_forward_dispatch<T: CpuFloat>(arguments: &[Ptr]) {
	unsafe { T::forward(cpu_argument::<u64>(arguments, 0) as *const T, cpu_argument::<u64>(arguments, 1) as *const T, cpu_argument::<u64>(arguments, 2) as *const *mut T, cpu_argument::<u64>(arguments, 3) as *const *mut T, cpu_argument::<u64>(arguments, 4) as *const i32, cpu_argument::<u64>(arguments, 5) as *const T, cpu_argument(arguments, 6), cpu_argument(arguments, 7), cpu_argument::<u64>(arguments, 12) as *mut u64, cpu_argument::<u64>(arguments, 13) as *const Tile) }
}
unsafe fn cpu_epoch_dispatch<T: CpuFloat>(a: &[Ptr]) {
	unsafe { T::epoch(cpu_argument::<u64>(a, 0) as *const T, cpu_argument::<u64>(a, 1) as *mut T, cpu_argument::<u64>(a, 2) as *const T, cpu_argument::<u64>(a, 3) as *mut T, cpu_argument::<u64>(a, 4) as *const u8, cpu_argument::<u64>(a, 5) as *mut T, cpu_argument::<u64>(a, 6) as *const *mut T, cpu_argument::<u64>(a, 7) as *const *mut T, cpu_argument::<u64>(a, 8) as *const *mut T, cpu_argument::<u64>(a, 9) as *const i32, cpu_argument::<u64>(a, 10) as *const T, cpu_argument::<u64>(a, 11) as *mut T, cpu_argument::<u64>(a, 12) as *mut T, cpu_argument::<u64>(a, 13) as *mut T, cpu_argument::<u64>(a, 14) as *mut T, cpu_argument::<u64>(a, 15) as *mut T, cpu_argument(a, 16), cpu_argument(a, 17), cpu_argument(a, 18), cpu_argument(a, 19), cpu_argument(a, 20), cpu_argument(a, 21), cpu_argument(a, 22), cpu_argument(a, 23), cpu_argument(a, 24), cpu_argument(a, 25), cpu_argument(a, 26), cpu_argument(a, 27), cpu_argument(a, 28), cpu_argument(a, 29), cpu_argument(a, 30), cpu_argument(a, 31), cpu_argument(a, 32), cpu_argument(a, 33), cpu_argument(a, 34), cpu_argument::<u64>(a, 35) as *mut u64, cpu_argument::<u64>(a, 36) as *const Tile) }
}
impl Gpu {
	#[cfg(any(amd, nvidia))]
	fn status(&self, status: i32, action: &str) -> Result<()> {
		driver_status(self.backend, status, action)
	}
	fn activate(&self) -> Result<()> {
		match &self.driver {
			Driver::Cpu => Ok(()),
			#[cfg(nvidia)]
			Driver::Cuda(driver) => self.status(unsafe { (driver.set)(driver.context) }, "context"),
			#[cfg(amd)]
			Driver::Hsa(_) => Ok(()),
			Driver::Remote(_) => Ok(()),
		}
	}
	fn kernels(&self, precision: FloatFormat) -> Result<(Dispatch, Dispatch)> {
		precision.kernel().and_then(|index| self.kernels[index]).ok_or_else(|| RecipeError::new(format!("{}({}) training is unavailable on {}", precision.family, precision.bits, self.name)))
	}
	fn shared_values(&self, precision: FloatFormat) -> Result<u32> {
		let (forward, epoch) = self.kernels(precision)?;
		let fixed = forward.kernel.shared.max(epoch.kernel.shared);
		let shared = self.shared_limit.checked_sub(fixed).ok_or_else(|| RecipeError::new("GPU kernel exceeds shared memory"))?;
		let shared_values = shared / precision.bytes() as u32;
		require(shared_values != 0, "GPU has no shared memory for contraction")?;
		Ok(shared_values)
	}
	fn tile_limit(&self, graph: &Graph, precision: FloatFormat) -> Result<Tile> {
		let shared_values = self.shared_values(precision)?;
		let m = graph.nodes.iter().map(|node| node.output.length).max().unwrap_or(graph.output.length).max(1);
		let n = graph.nodes.iter().map(|node| node.output.channels).max().unwrap_or(graph.output.channels).max(1);
		let k = graph.nodes.iter().map(|node| node.input.elements()).max().unwrap_or(graph.input.elements()).max(1);
		Ok(Tile { m: narrow(m, "contraction M tile limit")? as u32, n: narrow(n, "contraction N tile limit")? as u32, k: (narrow(k, "contraction K tile limit")? as u32).min(shared_values) })
	}
	fn proposal_limit(&self, values: [f64; 3], precision: FloatFormat) -> Result<Tile> {
		let dimension = |value: f64, name| -> Result<u32> {
			require(value.is_finite() && value >= 1.0 && value.fract() == 0.0, format!("RAT {name} must be a positive integer"))?;
			Ok(narrow(value as usize, name)? as u32)
		};
		Ok(Tile { m: dimension(values[0], "M")?, n: dimension(values[1], "N")?, k: dimension(values[2], "K")?.min(self.shared_values(precision)?) })
	}
	fn prior_tile(&self, graph: &Graph, precision: FloatFormat) -> Result<Tile> {
		let limit = self.tile_limit(graph, precision)?;
		let (forward, epoch) = self.kernels(precision)?;
		let block = forward.geometry.block.min(epoch.geometry.block);
		Ok(Tile { m: limit.m.min(block), n: limit.n.min(block), k: limit.k.min(block) })
	}
	#[cfg_attr(not(any(amd, nvidia)), allow(unused_unsafe))]
	fn allocate(&self, bytes: usize) -> Result<u64> {
		self.activate()?;
		unsafe {
			match &self.driver {
				Driver::Cpu => { let size = checked_add(bytes.max(1), size_of::<usize>(), "CPU allocation")?; let layout = std::alloc::Layout::from_size_align(size, 8).map_err(|error| RecipeError::new(format!("CPU allocation layout is invalid: {error}")))?; let base = std::alloc::alloc_zeroed(layout); require(!base.is_null(), "CPU allocation failed")?; base.cast::<usize>().write(size); Ok(base.add(size_of::<usize>()) as u64) }
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
					self.status((driver.allow)(1, &driver.cpu_agent, ptr::null(), pointer), "CPU allocation access")?;
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
				Driver::Cpu => { let base = (pointer as *mut u8).sub(size_of::<usize>()); let size = base.cast::<usize>().read(); std::alloc::dealloc(base, std::alloc::Layout::from_size_align_unchecked(size, 8)) },
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
				Driver::Cpu => { ptr::copy_nonoverlapping(src.cast::<u8>(), dst as *mut u8, bytes); Ok(()) }
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
				Driver::Cpu => { ptr::copy_nonoverlapping(src as *const u8, dst.cast::<u8>(), bytes); Ok(()) }
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
				Driver::Cpu => Ok(()),
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
		let dynamic = shared_bytes(tile.k, kernel.element)?;
		let shared = kernel.shared.checked_add(dynamic).ok_or_else(|| RecipeError::new("GPU shared memory size overflows"))?;
		require(shared <= self.shared_limit, "GPU shared memory exceeds its device limit")?;
		let _guard = self.dispatch.lock().map_err(|_| RecipeError::new("GPU dispatch lock is poisoned"))?;
		unsafe {
			match &self.driver {
				Driver::Cpu => { match kernel.object { 0 => cpu_forward_dispatch::<f64>(arguments), 1 => cpu_epoch_dispatch::<f64>(arguments), 2 => cpu_forward_dispatch::<f32>(arguments), 3 => cpu_epoch_dispatch::<f32>(arguments), _ => unreachable!() } Ok(()) },
				#[cfg(nvidia)]
				Driver::Cuda(driver) => {
					let stream = ptr::null_mut();
					self.status((driver.launch)(kernel.object as usize, threads / block, 1, 1, block, 1, 1, dynamic, stream, arguments.as_mut_ptr()), "dispatch")
				}
				#[cfg(amd)]
				Driver::Hsa(driver) => {
					require(arguments.len() == kernel.layout.len(), "kernel argument count is invalid")?;
					ptr::write_bytes(driver.kernarg.cast::<u8>(), 0, driver.kernarg_size);
					let mut offset = 0_usize;
					for (argument, kind) in arguments.iter().zip(kernel.layout) {
						let bytes = usize::from(*kind - b'0');
						offset = offset.next_multiple_of(bytes);
						ptr::copy_nonoverlapping((*argument).cast::<u8>(), driver.kernarg.cast::<u8>().add(offset), bytes);
						offset += bytes;
					}
					require(offset <= kernel.kernarg && kernel.kernarg <= driver.kernarg_size, "kernarg layout is invalid")?;
					(driver.store)(driver.signal, 1);
					let queue = &mut *(driver.queue as *mut HsaQueue);
					let index = (driver.write)(queue, 1);
					let packet = queue.base.cast::<HsaPacket>().add(index as usize & (queue.size as usize - 1));
					packet.write(HsaPacket { header: 0, setup: 1, workgroup_x: block as u16, workgroup_y: 1, workgroup_z: 1, reserved0: 0, grid_x: threads, grid_y: 1, grid_z: 1, private: kernel.private, group: shared, object: kernel.object, kernarg: driver.kernarg, reserved1: 0, completion: driver.signal });
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
fn cpu_device() -> Gpu {
	let dispatch = |object, element, layout| Dispatch { kernel: Kernel::remote(object, 0, element, layout), geometry: Geometry { groups: 1, block: 1 } };
	Gpu { name: "cpu".to_owned(), backend: Backend::Cpu, driver: Driver::Cpu, kernels: [Some((dispatch(0, 8, FORWARD_ARGS), dispatch(1, 8, EPOCH_ARGS))), Some((dispatch(2, 4, FORWARD_ARGS), dispatch(3, 4, EPOCH_F32_ARGS))), None], memory: u64::MAX, clock: 1, shared_limit: u32::MAX, dispatch: Mutex::new(()) }
}
fn devices() -> Result<&'static [Gpu]> {
	DEVICES
		.get_or_init(|| {
			if std::env::var_os("RECIPE_FORCE_CPU").is_some() { return Ok(vec![cpu_device()]) }
			let mut found = Vec::new();
			let mut errors = Vec::new();
			for load in [load_amd as fn() -> Result<Vec<Gpu>>, load_nvidia] {
				match load() {
					Ok(mut devices) => found.append(&mut devices),
					Err(error) => errors.push(error.to_string()),
				}
			}
			if found.is_empty() { if cfg!(any(amd, nvidia)) { return Err(RecipeError::new(errors.join("; "))) } found.push(cpu_device()) }
			Ok(found)
		})
		.as_ref()
		.map(Vec::as_slice)
		.map_err(Clone::clone)
}
fn device(name: Option<&str>) -> Result<&'static Gpu> {
	let found = devices()?;
	if let Some(name) = name {
		return found.iter().find(|gpu| gpu.name == name).ok_or_else(|| RecipeError::new(format!("GPU {name:?} is absent")));
	}
	require(found.len() == 1, "multiple GPUs require named selection")?;
	Ok(&found[0])
}
fn worker_list() -> Result<()> {
	let host = fs::read_to_string("/etc/hostname").map_err(|error| RecipeError::new(format!("cannot read hostname: {error}")))?;
	for gpu in devices()? {
		let (wide, f32) = (gpu.kernels[0].unwrap(), gpu.kernels[1]);
		println!("{}|{}|{:?}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}", host.trim(), gpu.name, gpu.backend, wide.0.geometry.groups, wide.0.geometry.block, wide.1.geometry.groups, wide.1.geometry.block, gpu.shared_limit, wide.0.kernel.shared, wide.1.kernel.shared, gpu.memory, gpu.clock, u8::from(f32.is_some()), f32.map_or(0, |value| value.0.geometry.groups), f32.map_or(0, |value| value.0.geometry.block), f32.map_or(0, |value| value.1.geometry.groups), f32.map_or(0, |value| value.1.geometry.block), f32.map_or(0, |value| value.0.kernel.shared), f32.map_or(0, |value| value.1.kernel.shared));
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
					let tile = Tile { m: get_u64(&mut input)? as u32, n: get_u64(&mut input)? as u32, k: get_u64(&mut input)? as u32 };
					let count = get_u64(&mut input)? as usize;
					let mut values = (0..count).map(|_| get_u64(&mut input)).collect::<Result<Vec<_>>>()?;
					let mut arguments = values.iter_mut().map(|value| value as *mut u64 as Ptr).collect::<Vec<_>>();
					let dispatch = gpu.kernels.get(usize::from(kernel[0]) / 2).and_then(|pair| *pair).map(|pair| if kernel[0] & 1 == 0 { pair.0 } else { pair.1 }).ok_or_else(|| RecipeError::new("remote Recipe kernel is unavailable"))?;
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
#[cfg_attr(target_os = "linux", unsafe(link_section = ".init_array"))]
#[cfg_attr(target_os = "macos", unsafe(link_section = "__DATA,__mod_init_func"))]
#[cfg_attr(target_os = "windows", unsafe(link_section = ".CRT$XCU"))]
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
	let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok_or_else(|| RecipeError::new("home directory is absent"))?;
	Ok(PathBuf::from(home).join(path))
}
fn ssh_hosts() -> Result<Vec<String>> {
	let text = fs::read_to_string(ssh_config()?).map_err(|error| RecipeError::new(format!("cannot read SSH config: {error}")))?;
	let mut hosts = text.lines().filter_map(|line| line.trim().strip_prefix("Host ")).flat_map(str::split_whitespace).filter(|host| !host.contains(['*', '?', '!'])).map(str::to_owned).collect::<Vec<_>>();
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
	forward_f32: Option<Geometry>,
	epoch_f32: Option<Geometry>,
	forward_f32_shared: u32,
	epoch_f32_shared: u32,
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
		remote_io("write to", worker.input.write_all(unsafe { std::slice::from_raw_parts(src.cast::<u8>(), bytes) }))?;
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
		for value in [u64::from(threads), u64::from(tile.m), u64::from(tile.n), u64::from(tile.k), arguments.len() as u64] {
			put_u64(&mut worker.input, value)?;
		}
		for (argument, kind) in arguments.iter().zip(dispatch.kernel.layout) {
			let mut value = u64::default().to_ne_bytes();
			unsafe { ptr::copy_nonoverlapping((*argument).cast::<u8>(), value.as_mut_ptr(), usize::from(*kind - b'0')) };
			put_u64(&mut worker.input, u64::from_ne_bytes(value))?;
		}
		remote_io("flush", worker.input.flush())?;
		read_response(&mut worker).map(|_| ())
	}
}
fn worker_process(host: &str, mode: &str) -> Result<Worker> {
	let executable = fs::read(std::env::current_exe().map_err(|error| RecipeError::new(format!("cannot locate Recipe executable: {error}")))?).map_err(|error| RecipeError::new(format!("cannot read Recipe executable: {error}")))?;
	let command = format!(concat!("worker=$(mktemp /tmp/recipe-worker.XXXXXX)\x3b trap 'rm -f \"$worker\"' EXIT\x3b ", "dd bs={} count=1 iflag=fullblock of=\"$worker\" status=none\x3b chmod 700 \"$worker\"\x3b ", "RECIPE_WORKER='{}' \"$worker\"",), executable.len(), mode);
	let mut child = Command::new("ssh").arg("-F").arg(ssh_config()?).args(["-o", "BatchMode=yes", "-o"]).arg(format!("ConnectTimeout={}", env!("RECIPE_SSH_CONNECT_TIMEOUT"))).arg(host).arg(command).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().map_err(|error| RecipeError::new(format!("cannot start Recipe worker on {host}: {error}")))?;
	let mut input = child.stdin.take().ok_or_else(|| RecipeError::new("Recipe worker stdin is absent"))?;
	let output = child.stdout.take().ok_or_else(|| RecipeError::new("Recipe worker stdout is absent"))?;
	input.write_all(&executable).map_err(|error| RecipeError::new(format!("cannot upload Recipe worker: {error}")))?;
	Ok(Worker { child, input, output })
}
fn remote_nodes(host: &str) -> Result<Vec<RemoteNode>> {
	let Worker { mut child, input, mut output } = worker_process(host, "list")?;
	drop(input);
	let mut text = String::new();
	output.read_to_string(&mut text).map_err(|error| RecipeError::new(format!("cannot read Recipe worker: {error}")))?;
	require(child.wait().map_err(|error| RecipeError::new(format!("cannot wait for {host}: {error}")))?.success(), format!("Recipe worker on {host} failed"))?;
	text.lines()
		.map(|line| {
			let fields = line.split('|').collect::<Vec<_>>();
			require(fields.len() == 19, "Recipe worker device is invalid")?;
			let backend = match fields[2] {
				"Amd" => Backend::Amd,
				"Nvidia" => Backend::Nvidia,
				_ => return Err(RecipeError::new("Recipe worker backend is invalid")),
			};
			let positive = |index| -> Result<u32> { Ok(narrow(natural("remote device value", fields[index])?, "remote device value")? as u32) };
			let fixed = |index: usize| fields[index].parse::<u32>().map_err(|error| RecipeError::new(format!("invalid remote device value: {error}")));
			let f32 = match fields[12] {
				"0" => false,
				"1" => true,
				_ => return Err(RecipeError::new("remote fp32 capability is invalid")),
			};
			let geometry = |groups, block| -> Result<Geometry> { Ok(Geometry { groups: positive(groups)?, block: positive(block)? }) };
			Ok(RemoteNode { info: DeviceInfo { name: format!("{}:{}", fields[0], fields[1]), host: fields[0].to_owned() }, transport: host.to_owned(), device: fields[1].to_owned(), backend, forward: Geometry { groups: positive(3)?, block: positive(4)? }, epoch: Geometry { groups: positive(5)?, block: positive(6)? }, shared_limit: positive(7)?, forward_shared: fixed(8)?, epoch_shared: fixed(9)?, memory: fields[10].parse().map_err(|error| RecipeError::new(format!("invalid remote memory: {error}")))?, clock: natural("remote timestamp frequency", fields[11])? as u64, forward_f32: if f32 { Some(geometry(13, 14)?) } else { None }, epoch_f32: if f32 { Some(geometry(15, 16)?) } else { None }, forward_f32_shared: fixed(17)?, epoch_f32_shared: fixed(18)? })
		})
		.collect()
}
fn host_has_gpu(host: &str) -> Result<bool> {
	Ok(Command::new("ssh").arg("-F").arg(ssh_config()?).args(["-o", "BatchMode=yes", "-o"]).arg(format!("ConnectTimeout={}", env!("RECIPE_SSH_CONNECT_TIMEOUT"))).arg(host).arg("test -d /sys/class/kfd/kfd/topology/nodes || command -v nvidia-smi >/dev/null").stdout(Stdio::null()).stderr(Stdio::null()).status().map_err(|error| RecipeError::new(format!("cannot probe {host}: {error}")))?.success())
}
static REMOTE_NODES: OnceLock<Result<Vec<RemoteNode>>> = OnceLock::new();
fn reachable_nodes() -> Result<&'static [RemoteNode]> {
	REMOTE_NODES
		.get_or_init(|| {
			let hosts = ssh_hosts()?;
			let capable = std::thread::scope(|scope| {
				let workers = hosts.iter().map(|host| scope.spawn(|| host_has_gpu(host))).collect::<Vec<_>>();
				workers.into_iter().map(|worker| worker.join().map_err(|_| RecipeError::new("GPU host probe panicked"))?).collect::<Result<Vec<_>>>()
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
	let wide = (Dispatch { kernel: Kernel::remote(0, node.forward_shared, 8, FORWARD_ARGS), geometry: node.forward }, Dispatch { kernel: Kernel::remote(1, node.epoch_shared, 8, EPOCH_ARGS), geometry: node.epoch });
	let f32 = node.forward_f32.zip(node.epoch_f32).map(|geometry| (Dispatch { kernel: Kernel::remote(2, node.forward_f32_shared, 4, FORWARD_ARGS), geometry: geometry.0 }, Dispatch { kernel: Kernel::remote(3, node.epoch_f32_shared, 4, EPOCH_F32_ARGS), geometry: geometry.1 }));
	Ok(Gpu { name: node.info.name.clone(), backend: node.backend, driver: Driver::Remote(Remote { io: Mutex::new(worker) }), kernels: [Some(wide), f32, None], memory: node.memory, clock: node.clock, shared_limit: node.shared_limit, dispatch: Mutex::new(()) })
}
static REMOTE_GPUS: OnceLock<Result<Vec<Gpu>>> = OnceLock::new();
fn remote_gpus() -> Result<&'static [Gpu]> {
	REMOTE_GPUS.get_or_init(|| reachable_nodes()?.iter().map(remote_gpu).collect()).as_ref().map(Vec::as_slice).map_err(Clone::clone)
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
	let host = fs::read_to_string("/etc/hostname").map_err(|error| RecipeError::new(format!("cannot read hostname: {error}")))?;
	let host = host.trim().to_owned();
	let mut nodes = found.iter().map(|gpu| DeviceInfo { name: format!("{host}:{}", gpu.name), host: host.clone() }).collect::<Vec<_>>();
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
			links.push(DeviceLink { from: name(source), to: name(target), latency_ms: latency * 1000.0, bytes_per_second: bandwidth_bytes as f64 / duration });
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
unsafe fn hsa_kernel(symbol: HsaSymbol, info: HsaSymbolInfo, executable: u64, agent: u64, name: &'static [u8], element: u8, layout: &'static [u8]) -> Result<Kernel> {
	let mut handle = 0;
	driver_status(Backend::Amd, unsafe { symbol(executable, name.as_ptr(), &agent, &mut handle) }, "kernel lookup")?;
	let mut kernel = Kernel { object: 0, shared: 0, element, kernarg: 0, private: 0, layout };
	for (attribute, output) in [(22, (&mut kernel.object as *mut u64).cast()), (11, (&mut kernel.kernarg as *mut usize).cast()), (13, (&mut kernel.shared as *mut u32).cast()), (14, (&mut kernel.private as *mut u32).cast())] {
		driver_status(Backend::Amd, unsafe { info(handle, attribute, output) }, "kernel metadata")?;
	}
	Ok(kernel)
}
#[cfg(amd)]
fn kfd_property(text: &str, name: &str) -> Result<u32> {
	text.lines().find_map(|line| line.split_once(' ').filter(|value| value.0 == name)).ok_or_else(|| RecipeError::new(format!("KFD property {name:?} is absent")))?.1.parse::<u32>().map_err(|error| RecipeError::new(format!("KFD property {name:?} is invalid: {error}")))
}
#[cfg(amd)]
include!(concat!(env!("OUT_DIR"), "/hsa-embed.rs"));
#[cfg(amd)]
fn hsa_artifact(artifacts: &'static [(&str, &[u8])], target: &str) -> Result<&'static [u8]> {
	artifacts.iter().find_map(|entry| (entry.0 == target).then_some(entry.1)).ok_or_else(|| RecipeError::new(format!("HSA artifact for {target} is absent")))
}
fn load_amd() -> Result<Vec<Gpu>> {
	#[cfg(not(amd))]
	return Err(RecipeError::new("AMD support is not compiled into this build"));
	#[cfg(amd)]
	unsafe {
		let runtime = Library::open(env!("RECIPE_HSA_RUNTIME"))?;
		let init: unsafe extern "C" fn() -> i32 = runtime.function(b"hsa_init\0")?;
		let iterate: unsafe extern "C" fn(extern "C" fn(u64, Ptr) -> i32, Ptr) -> i32 = runtime.function(b"hsa_iterate_agents\0")?;
		let info: HsaInfo = runtime.function(b"hsa_agent_get_info\0")?;
		let check = |s, a| driver_status(Backend::Amd, s, a);
		check(init(), "initialization")?;
		let mut cpu = HsaQuery { info, attribute: 17, expected: 0, secondary: -1, mask: 0, found: 0 };
		let mut gpu = HsaGpuQuery { info, found: Vec::new() };
		check(iterate(collect_hsa, (&mut cpu as *mut HsaQuery).cast()), "CPU agent")?;
		check(iterate(collect_discrete_hsa, (&mut gpu as *mut HsaGpuQuery).cast()), "GPU agent")?;
		require(cpu.found != 0 && !gpu.found.is_empty(), "AMD CPU or discrete GPU agent is absent")?;
		gpu.found.into_iter().enumerate().map(|(index, agent)| load_amd_gpu(&runtime, info, cpu.found, agent, index)).collect()
	}
}
#[cfg(amd)]
fn load_amd_gpu(runtime: &Library, info: HsaInfo, cpu_agent: u64, agent: u64, index: usize) -> Result<Gpu> {
	unsafe {
		let pool_info: HsaInfo = runtime.function(b"hsa_amd_memory_pool_get_info\0")?;
		let pool_iterate: unsafe extern "C" fn(u64, extern "C" fn(u64, Ptr) -> i32, Ptr) -> i32 = runtime.function(b"hsa_amd_agent_iterate_memory_pools\0")?;
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
		for (attribute, output, action) in [(6, (&mut wave as *mut u32).cast(), "wave query"), (8, (&mut workgroup as *mut u32).cast(), "workgroup query"), (0xA002, (&mut available as *mut u32).cast(), "CU query"), (0xA004, (&mut node as *mut u32).cast(), "KFD node query"), (0xA014, (&mut cus as *mut u32).cast(), "cooperative CU query")] {
			check(info(agent, attribute, output), action)?;
		}
		require(cus <= available, "AMD cooperative CU count exceeds available CUs")?;
		let path = format!("/sys/class/kfd/kfd/topology/nodes/{node}/properties");
		let properties = fs::read_to_string(&path).map_err(|error| RecipeError::new(format!("cannot read {path}: {error}")))?;
		let gfx = kfd_property(&properties, "gfx_target_version")?;
		let target = format!("gfx{}{}{}", gfx / 10000, gfx / 100 % 100, gfx % 100);
		let codes = [hsa_artifact(HSA_CODE_OBJECTS, &target)?, hsa_artifact(HSA_F32_CODE_OBJECTS, &target)?, hsa_artifact(HSA_F16_CODE_OBJECTS, &target)?];
		let reader_create: unsafe extern "C" fn(*const c_void, usize, *mut u64) -> i32 = runtime.function(b"hsa_code_object_reader_create_from_memory\0")?;
		let executable_create: unsafe extern "C" fn(i32, i32, Ptr, *mut u64) -> i32 = runtime.function(b"hsa_executable_create_alt\0")?;
		let executable_load: unsafe extern "C" fn(u64, u64, u64, Ptr, Ptr) -> i32 = runtime.function(b"hsa_executable_load_agent_code_object\0")?;
		let executable_freeze: unsafe extern "C" fn(u64, Ptr) -> i32 = runtime.function(b"hsa_executable_freeze\0")?;
		let symbol: HsaSymbol = runtime.function(b"hsa_executable_get_symbol_by_name\0")?;
		let symbol_info: HsaSymbolInfo = runtime.function(b"hsa_executable_symbol_get_info\0")?;
		let (mut readers, mut executable) = ([0; 3], 0);
		for (index, code) in codes.into_iter().enumerate() { check(reader_create(code.as_ptr().cast(), code.len(), &mut readers[index]), "code-object reader")? }
		check(executable_create(1, 0, ptr::null_mut(), &mut executable), "executable creation")?;
		for reader in readers { check(executable_load(executable, agent, reader, ptr::null_mut(), ptr::null_mut()), "code-object load")? }
		check(executable_freeze(executable, ptr::null_mut()), "executable freeze")?;
		let lds = kfd_property(&properties, "lds_size_in_kb")?.checked_mul(1024).ok_or_else(|| RecipeError::new("AMD LDS size overflows"))?;
		let specifications: [(&[u8], u8, &[u8]); 6] = [(b"forward_graph.kd\0", 8, FORWARD_ARGS), (b"tape_epoch_graph.kd\0", 8, EPOCH_ARGS), (b"forward_graph_f32.kd\0", 4, FORWARD_ARGS), (b"tape_epoch_graph_f32.kd\0", 4, EPOCH_F32_ARGS), (b"forward_graph_f16.kd\0", 2, FORWARD_ARGS), (b"tape_epoch_graph_f16.kd\0", 2, EPOCH_F16_ARGS)];
		let mut dispatches = Vec::new();
		for (name, element, layout) in specifications { let kernel = hsa_kernel(symbol, symbol_info, executable, agent, name, element, layout)?; let geometry = amd(cus, wave, workgroup, lds, Resources { shared: kernel.shared, max_block: workgroup, element })?; dispatches.push(Dispatch { kernel, geometry }) }
		let queue_create: unsafe extern "C" fn(u64, u32, u32, Ptr, Ptr, u32, u32, *mut Ptr) -> i32 = runtime.function(b"hsa_queue_create\0")?;
		let signal_create: unsafe extern "C" fn(i64, u32, *const u64, *mut u64) -> i32 = runtime.function(b"hsa_signal_create\0")?;
		let allocate: unsafe extern "C" fn(u64, usize, u32, *mut Ptr) -> i32 = runtime.function(b"hsa_amd_memory_pool_allocate\0")?;
		let allow: unsafe extern "C" fn(u32, *const u64, *const u32, *const c_void) -> i32 = runtime.function(b"hsa_amd_agents_allow_access\0")?;
		let (ka_size, mut ka) = (dispatches.iter().map(|dispatch| dispatch.kernel.kernarg).max().unwrap(), ptr::null_mut());
		let (mut queue, mut completion) = (ptr::null_mut(), 0);
		driver_status(Backend::Amd, queue_create(agent, 256, 2, ptr::null_mut(), ptr::null_mut(), u32::MAX, u32::MAX, &mut queue), "queue creation")?;
		check(signal_create(0, 0, ptr::null(), &mut completion), "signal creation")?;
		check(allocate(kernarg.found, ka_size, 0, &mut ka), "KERNARG allocation")?;
		check(allow(1, &agent, ptr::null(), ka), "GPU KERNARG access")?;
		let mut dispatches = dispatches.into_iter();
		let kernels = std::array::from_fn(|_| Some((dispatches.next().unwrap(), dispatches.next().unwrap())));
		eprintln!("AMD forward block {} epoch block {}", kernels[0].unwrap().0.geometry.block, kernels[0].unwrap().1.geometry.block);
		Ok(Gpu { name: format!("amd{index}"), backend: Backend::Amd, driver: Driver::Hsa(Hsa { allocate, allow, queue, cpu_agent, kernarg_size: ka_size, kernarg: ka, free: runtime.function(b"hsa_amd_memory_pool_free\0")?, copy: runtime.function(b"hsa_memory_copy\0")?, store: runtime.function(b"hsa_signal_store_screlease\0")?, wait: runtime.function(b"hsa_signal_wait_scacquire\0")?, write: runtime.function(b"hsa_queue_add_write_index_scacq_screl\0")?, signal: completion, vram_pool: vram.found, kernarg_pool: kernarg.found }), kernels, memory: memory as u64, clock, shared_limit: lds, dispatch: Mutex::new(()) })
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
		let runtime = Library::open(if cfg!(windows) { "nvcuda.dll" } else { env!("RECIPE_NV_RUNTIME") })?;
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
			let (mut forward, mut epoch, mut forward_f32, mut epoch_f32) = (0, 0, 0, 0);
			let mut context = ptr::null_mut();
			let (mut cus, mut wave, mut workgroup, mut block_lds, mut sm_lds, mut registers, mut threads, mut cooperative) = (0, 0, 0, 0, 0, 0, 0, 0);
			let mut memory = 0;
			check(total(&mut memory, device), "VRAM size")?;
			for (kind, output, action) in [(CUS, &mut cus, "SM query"), (WAVE, &mut wave, "warp query"), (MAX_BLOCK, &mut workgroup, "workgroup query"), (BLOCK_LDS, &mut block_lds, "workgroup LDS query"), (SM_LDS, &mut sm_lds, "SM LDS query"), (REGISTERS_PER_SM, &mut registers, "register query"), (THREADS_PER_SM, &mut threads, "resident thread query"), (COOPERATIVE, &mut cooperative, "cooperative launch query")] {
				check(attribute(output, kind, device), action)?;
			}
			require(cooperative != 0, "Nvidia device does not support cooperative launch")?;
			check(create(&mut context, 0, device), "context creation")?;
			let module = |bytes: &[u8]| -> Result<Ptr> {
				let image = std::ffi::CString::new(bytes).map_err(|error| RecipeError::new(format!("Nvidia PTX contains a zero byte: {error}")))?;
				let mut module = ptr::null_mut();
				check(load(&mut module, image.as_ptr().cast()), "module load")?;
				Ok(module)
			};
			let wide = module(include_bytes!(env!("RECIPE_NV_PTX")))?;
			let float = module(include_bytes!(env!("RECIPE_NV_F32_PTX")))?;
			for (output, module, name, action) in [(&mut forward, wide, b"forward_graph\0".as_ptr(), "forward"), (&mut epoch, wide, b"tape_epoch_graph\0".as_ptr(), "epoch"), (&mut forward_f32, float, b"forward_graph_f32\0".as_ptr(), "fp32 forward"), (&mut epoch_f32, float, b"tape_epoch_graph_f32\0".as_ptr(), "fp32 epoch")] {
				check(function(output, module, name), action)?;
			}
			let resource = |kernel, element| -> Result<(Resources, u32)> {
				let (mut max_block, mut shared, mut used_registers) = (0, 0, 0);
				for (kind, output, action) in [(0, &mut max_block, "kernel workgroup query"), (1, &mut shared, "kernel LDS query"), (4, &mut used_registers, "kernel register query")] {
					check(function_attribute(output, kind, kernel), action)?;
				}
				require(max_block > 0 && shared >= 0 && used_registers > 0, "Nvidia kernel resources are invalid")?;
				Ok((Resources { shared: shared as u32, max_block: max_block as u32, element }, used_registers as u32))
			};
			let (forward_resource, forward_registers) = resource(forward, 8)?;
			let (epoch_resource, epoch_registers) = resource(epoch, 8)?;
			let (forward_f32_resource, forward_f32_registers) = resource(forward_f32, 4)?;
			let (epoch_f32_resource, epoch_f32_registers) = resource(epoch_f32, 4)?;
			let geometry = |resources: Resources, used_registers: u32| -> Result<Geometry> {
				let register_wave = used_registers.checked_mul(wave as u32).ok_or_else(|| RecipeError::new("Nvidia wave register count overflows"))?;
				let observed = (registers as u32 / register_wave).min(threads as u32 / wave as u32);
				nvidia(cus as u32, wave as u32, workgroup as u32, block_lds as u32, sm_lds as u32, observed, resources)
			};
			let forward_geometry = geometry(forward_resource, forward_registers)?;
			let epoch_geometry = geometry(epoch_resource, epoch_registers)?;
			let forward_f32_geometry = geometry(forward_f32_resource, forward_f32_registers)?;
			let epoch_f32_geometry = geometry(epoch_f32_resource, epoch_f32_registers)?;
			for (kernel, geometry, action) in [(forward, forward_geometry, "forward occupancy"), (epoch, epoch_geometry, "epoch occupancy"), (forward_f32, forward_f32_geometry, "fp32 forward occupancy"), (epoch_f32, epoch_f32_geometry, "fp32 epoch occupancy")] {
				let mut active = 0;
				driver_status(Backend::Nvidia, occupancy(&mut active, kernel, geometry.block as i32, 0), action)?;
				require(active > 0, format!("Nvidia {action} has no resident workgroup"))?;
			}
			let cuda = Cuda { context, set: runtime.function(b"cuCtxSetCurrent\0")?, allocate: runtime.function(b"cuMemAlloc_v2\0")?, free: runtime.function(b"cuMemFree_v2\0")?, upload: runtime.function(b"cuMemcpyHtoD_v2\0")?, download: runtime.function(b"cuMemcpyDtoH_v2\0")?, synchronize: runtime.function(b"cuCtxSynchronize\0")?, launch: runtime.function(b"cuLaunchCooperativeKernel\0")? };
			eprintln!("Nvidia forward block {} epoch block {}", forward_geometry.block, epoch_geometry.block);
			Ok(Gpu { name: format!("nv{index}"), backend: Backend::Nvidia, driver: Driver::Cuda(cuda), kernels: [Some((Dispatch { kernel: Kernel::cuda(forward, forward_resource.shared, 8, FORWARD_ARGS), geometry: forward_geometry }, Dispatch { kernel: Kernel::cuda(epoch, epoch_resource.shared, 8, EPOCH_ARGS), geometry: epoch_geometry })), Some((Dispatch { kernel: Kernel::cuda(forward_f32, forward_f32_resource.shared, 4, FORWARD_ARGS), geometry: forward_f32_geometry }, Dispatch { kernel: Kernel::cuda(epoch_f32, epoch_f32_resource.shared, 4, EPOCH_F32_ARGS), geometry: epoch_f32_geometry })), None], memory: memory as u64, clock: NANOSECOND_TIMER_HZ, shared_limit: (block_lds as u32).min(sm_lds as u32), dispatch: Mutex::new(()) })
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
#[cfg(all(any(amd, nvidia), not(windows)))]
#[link(name = "dl")]
unsafe extern "C" {
	fn dlopen(name: *const std::ffi::c_char, flags: i32) -> Ptr;
	fn dlsym(handle: Ptr, name: *const std::ffi::c_char) -> Ptr;
}
#[cfg(all(nvidia, windows))]
unsafe fn dlopen(name: *const std::ffi::c_char, _: i32) -> Ptr {
	unsafe { LoadLibraryA(name) }
}
#[cfg(all(nvidia, windows))]
unsafe fn dlsym(handle: Ptr, name: *const std::ffi::c_char) -> Ptr {
	unsafe { GetProcAddress(handle, name) }
}
#[cfg(all(nvidia, windows))]
#[link(name = "kernel32")]
unsafe extern "system" {
	fn LoadLibraryA(name: *const std::ffi::c_char) -> Ptr;
	fn GetProcAddress(handle: Ptr, name: *const std::ffi::c_char) -> Ptr;
}
unsafe extern "C" {
	fn signal(number: i32, handler: extern "C" fn(i32)) -> usize;
	#[cfg_attr(windows, link_name = "_write")]
	fn write(file: i32, bytes: *const c_void, length: usize) -> isize;
}
unsafe extern "C" {
	fn recipe_forward_cpu(samples: *const f64, weights: *const f64, value_pointers: *const *mut f64, context_pointers: *const *mut f64, descriptors: *const i32, arguments: *const f64, rows: i32, nodes: i32, threads: i32, tile_m: i32, tile_n: i32, tile_k: i32, timings: *mut u64, tiles: *const Tile);
	fn recipe_forward_cpu_f32(samples: *const f32, weights: *const f32, value_pointers: *const *mut f32, context_pointers: *const *mut f32, descriptors: *const i32, arguments: *const f32, rows: i32, nodes: i32, threads: i32, tile_m: i32, tile_n: i32, tile_k: i32, timings: *mut u64, tiles: *const Tile);
	fn recipe_epoch_cpu(samples: *const f64, input_adjoint: *mut f64, targets: *const f64, weights: *mut f64, frozen: *const u8, best: *mut f64, values: *const *mut f64, contexts: *const *mut f64, adjoints: *const *mut f64, descriptors: *const i32, arguments: *const f64, metrics: *mut f64, gradient: *mut f64, moments: *mut f64, variances: *mut f64, best_loss: *mut f64, rows: i32, nodes: i32, parameters: i32, loss: i32, huber: f64, rate: f64, beta1: f64, beta2: f64, beta1_power: f64, beta2_power: f64, epsilon: f64, decay: f64, tolerance: f64, step: i32, threads: i32, tile_m: i32, tile_n: i32, tile_k: i32, phase: i32, timings: *mut u64, tiles: *const Tile);
	fn recipe_epoch_cpu_f32(samples: *const f32, input_adjoint: *mut f32, targets: *const f32, weights: *mut f32, frozen: *const u8, best: *mut f32, values: *const *mut f32, contexts: *const *mut f32, adjoints: *const *mut f32, descriptors: *const i32, arguments: *const f32, metrics: *mut f32, gradient: *mut f32, moments: *mut f32, variances: *mut f32, best_loss: *mut f32, rows: i32, nodes: i32, parameters: i32, loss: i32, huber: f32, rate: f32, beta1: f32, beta2: f32, beta1_power: f32, beta2_power: f32, epsilon: f32, decay: f32, tolerance: f32, step: i32, threads: i32, tile_m: i32, tile_n: i32, tile_k: i32, phase: i32, timings: *mut u64, tiles: *const Tile);
}
fn distance(left: &[f64], right: &[f64]) -> f64 {
	left.iter().zip(right).map(|(a, b)| (a - b).powi(2)).sum()
}
fn nearest(query: &[f64], state: &[f64], features: usize) -> (usize, f64) {
	state.chunks_exact(features).enumerate().map(|(index, row)| (index, distance(query, row))).min_by(|left, right| left.1.total_cmp(&right.1)).unwrap_or((0, f64::INFINITY))
}
fn graph_inputs(graph: &Graph, samples: &[f64], targets: &[f64], rows: usize, gpu: &'static Gpu, precision: FloatFormat) -> Result<Vec<f64>> {
	let input_count = checked_mul(rows, graph.input.elements(), "estimator input slice")?;
	if graph.nodes.is_empty() {
		return Ok(samples[..rows * graph.output.elements()].to_vec());
	}
	let mut tape = GpuTape::new(graph, &samples[..input_count], &targets[..rows], gpu, precision)?;
	tape.forward()?;
	tape.predictions()
}
fn fit_surrogate(input: Shape, samples: &[f64], targets: &[f64], hidden: usize, gpu: &'static Gpu, config: Config) -> Result<Graph> {
	require(!targets.is_empty(), "surrogate requires teacher outputs")?;
	let sample_count = checked_mul(targets.len(), input.elements(), "surrogate samples")?;
	require(samples.len() == sample_count, "surrogate sample batch is invalid")?;
	let model = recipe.model().layer(hidden).tanh().layer(1);
	let prepared = Prepared { samples: samples.to_vec(), targets: targets.to_vec(), rows: targets.len(), features: input.elements(), schema: String::new(), sequence: None, norm_mean: Vec::new(), norm_scale: Vec::new(), identities: Vec::new() };
	let mut graph = compile_output(&model, &prepared, prepared.rows, gpu, config, 1)?;
	let mut tape = GpuTape::new(&graph, samples, targets, gpu, config.precision)?;
	for _ in 0..config.surrogate_epochs {
		tape.advance()?;
		tape.epoch(config.surrogate_rate, mse, 0.0, config, false)?;
	}
	tape.capture(&mut graph)?;
	graph.frozen.fill(1);
	Ok(graph)
}
type Predictor = Box<dyn Fn(usize, &[f64]) -> f64>;
fn cluster(data: &[f64], width: usize, clusters: usize, iterations: usize, importance: Option<&[f64]>) -> Result<(Vec<f64>, Vec<usize>)> {
	let rows = data.len() / width;
	require(width != 0 && clusters != 0 && clusters <= rows, "kmeans cluster count is invalid")?;
	let (mut centers, mut assignments, mut distances) = (data[..clusters * width].to_vec(), vec![0; rows], vec![0.0; rows]);
	for _ in 0..iterations {
		for (row, sample) in data.chunks_exact(width).enumerate() {
			let selected = nearest(sample, &centers, width);
			assignments[row] = selected.0;
			distances[row] = selected.1;
		}
		for group in 0..clusters {
			let members = assignments.iter().enumerate().filter(|value| *value.1 == group).map(|value| value.0).collect::<Vec<_>>();
			if members.is_empty() {
				let worst = distances.iter().enumerate().max_by(|a, b| a.1.total_cmp(b.1)).map(|value| value.0).ok_or_else(|| RecipeError::new("kmeans has no training row"))?;
				centers[group * width..(group + 1) * width].copy_from_slice(&data[worst * width..(worst + 1) * width]);
				distances[worst] = -1.0;
			} else {
				for feature in 0..width {
					let total = members.iter().map(|&row| importance.map_or(1.0, |weights| weights[row])).sum::<f64>();
					centers[group * width + feature] = members.iter().map(|&row| data[row * width + feature] * importance.map_or(1.0, |weights| weights[row])).sum::<f64>() / total;
				}
			}
		}
	}
	Ok((centers, assignments))
}
fn fit_kmeans(clusters: usize, data: &Prepared, rows: usize, config: Config, _: bool) -> Result<Predictor> {
	let (centers, _) = cluster(&data.samples[..rows * data.features], data.features, clusters, config.kmeans_iterations, None)?;
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
		let mut nearest = samples.chunks_exact(features).enumerate().filter(|value| !exclude || value.0 != row).map(|value| (distance(query, value.1), targets[value.0])).collect::<Vec<_>>();
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
fn shared_bytes(tile: u32, element: u8) -> Result<u32> {
	tile.max(DOUBLE_BUFFER_VALUES).checked_mul(u32::from(element)).ok_or_else(|| RecipeError::new("GPU shared memory size overflows"))
}
#[cfg(any(amd, nvidia))]
#[derive(Clone, Copy)]
struct Resources {
	pub shared: u32,
	pub max_block: u32,
	pub element: u8,
}
#[derive(Clone, Copy)]
struct Geometry { pub groups: u32, pub block: u32 }
impl Geometry {
	pub fn threads(self, work: u32) -> Result<u32> {
		self.groups.min(work.div_ceil(self.block)).checked_mul(self.block).filter(|value| *value != 0).ok_or_else(|| RecipeError::new("GPU launch size overflows"))
	}
}
#[cfg(any(amd, nvidia))]
fn geometry(cus: u32, wave: u32, workgroup: u32, lds: u32, groups_per_cu: u32, resources: Resources) -> Result<Geometry> {
	require(wave != 0 && wave <= workgroup && wave <= resources.max_block, "GPU wave exceeds kernel workgroup")?;
	let waves = groups_per_cu.min(workgroup / wave).min(resources.max_block / wave);
	require(waves != 0, "GPU has no resident wave")?;
	let block = waves.checked_mul(wave).ok_or_else(|| RecipeError::new("GPU workgroup size overflows"))?;
	let shared = resources.shared.checked_add(shared_bytes(0, resources.element)?).ok_or_else(|| RecipeError::new("GPU shared memory size overflows"))?;
	require(shared <= lds, "GPU tile exceeds local memory")?;
	Ok(Geometry { groups: cus, block })
}
#[cfg(amd)]
fn amd(cus: u32, wave: u32, workgroup: u32, lds: u32, resources: Resources) -> Result<Geometry> {
	geometry(cus, wave, workgroup, lds, u32::from(wave != 0), resources)
}
#[cfg(nvidia)]
fn nvidia(cus: u32, wave: u32, workgroup: u32, block_lds: u32, sm_lds: u32, waves_per_cu: u32, resources: Resources) -> Result<Geometry> {
	let shared = resources.shared.checked_add(shared_bytes(0, resources.element)?).ok_or_else(|| RecipeError::new("Nvidia shared memory size overflows"))?;
	require(shared <= block_lds, "Nvidia tile exceeds workgroup shared memory")?;
	geometry(cus, wave, workgroup, sm_lds, waves_per_cu, resources)
}
pub trait IntoDataSources {
	fn into_data_sources(self) -> Vec<String>;
}
impl IntoDataSources for &str {
	fn into_data_sources(self) -> Vec<String> {
		vec![self.to_owned()]
	}
}
impl IntoDataSources for String {
	fn into_data_sources(self) -> Vec<String> {
		vec![self]
	}
}
impl<T: Into<String>, const N: usize> IntoDataSources for [T; N] {
	fn into_data_sources(self) -> Vec<String> {
		self.into_iter().map(Into::into).collect()
	}
}
impl<T: Into<String>> IntoDataSources for Vec<T> {
	fn into_data_sources(self) -> Vec<String> {
		self.into_iter().map(Into::into).collect()
	}
}
impl<T: Clone + Into<String>> IntoDataSources for &[T] {
	fn into_data_sources(self) -> Vec<String> {
		self.iter().cloned().map(Into::into).collect()
	}
}
impl Data {
	pub fn target(mut self, target: impl IntoDataSources) -> Self {
		self.target = target.into_data_sources();
		self
	}
	pub fn r#in(mut self, names: impl IntoDataSources) -> Self {
		self.routes.push(Route { inputs: names.into_data_sources(), outputs: Vec::new() });
		self
	}
	pub fn out(mut self, names: impl IntoDataSources) -> Self {
		self.routes.last_mut().unwrap_or_else(|| panic!(".out() requires a preceding .r#in()")).outputs = names.into_data_sources();
		self
	}
	pub fn exclude(mut self, names: impl IntoDataSources) -> Self {
		self.exclusions = names.into_data_sources();
		self
	}
	pub fn set(mut self, source: impl Into<String>) -> Self {
		self.sources.push(source.into());
		self
	}
	pub const fn norm(mut self, _: ZScore) -> Self {
		self.normalize = true;
		self
	}
	pub const fn split(mut self, fraction: f64) -> Self {
		self.split = fraction;
		self
	}
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
struct ChildTable { name: String, headers: Vec<String>, rows: usize }
struct Table { name: String, headers: Vec<String>, rows: Vec<Vec<String>>, children: Vec<ChildTable> }
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
	name == header || name == format!("{}.{}", table.name, header) || name == format!("col{}", column + 1) || name == format!("{}.col{}", table.name, column + 1) || header.strip_suffix(name).is_some_and(|prefix| prefix.ends_with('.')) || header.rsplit_once('.').is_some_and(|(base, row)| row.parse::<usize>().is_ok() && (base == name || base.strip_suffix(name).is_some_and(|prefix| prefix.ends_with('.'))))
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
			let tag = if tagged(targets) {
				"\x1b[32m[target]\x1b[0m  "
			} else if tagged(exclusions) {
				"\x1b[31m[excluded]\x1b[0m"
			} else {
				"          "
			};
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
		let bytes = fs::read(&path).map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
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
	let repeated = columns.iter().all(|column| tables[column.0].headers[column.1].rsplit_once('.').and_then(|value| value.1.parse::<usize>().ok().map(|row| *sequence_widths.entry(row).or_insert(0) += column.2.width())).is_some());
	let sequence = (repeated && sequence_widths.len() > 1 && sequence_widths.keys().copied().eq(1..=sequence_widths.len()) && sequence_widths.values().all(|width| *width == sequence_widths[&1])).then(|| Shape { channels: sequence_widths[&1], length: sequence_widths.len() });
	require(features != 0, "dataset has no training features")?;
	let target_categories = selected.iter().map(|target| categories(&tables[target.0], target.1, row_count)).collect::<Vec<_>>();
	let mut samples = Vec::new();
	let mut targets = Vec::new();
	let mut missing = vec![0_usize; columns.len()];
	for row in 0..row_count {
		let mut encoded = Vec::with_capacity(features);
		let valid = columns.iter().all(|column| tables[column.0].rows[row].get(column.1).is_some_and(|value| encode(value, &column.2, &mut encoded)));
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
				let target = value.and_then(|value| value.parse::<f64>().ok()).or_else(|| value.and_then(|value| categories.iter().position(|category| category == value)).map(|value| value as f64));
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
	let mut identities = samples.chunks_exact(features).zip(&targets).enumerate().map(|(row, (sample, target))| sample_identity(sample, *target, row)).collect();
	shuffle(&mut samples, &mut targets, &mut identities, features)?;
	let (norm_mean, norm_scale) = if data.normalize {
		normalize_samples(&mut samples, features, ((rows as f64) * data.split).floor() as usize)?
	} else {
		impute_missing(&mut samples);
		(Vec::new(), Vec::new())
	};
	let schema = columns.iter().map(|column| format!("{}.{}:{}", tables[column.0].name, tables[column.0].headers[column.1], column.2.width())).collect::<Vec<_>>().join("|") + "->" + &data.target.join("|");
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
	for value in samples.iter_mut() {
		if !value.is_finite() {
			*value = 0.0
		}
	}
}
fn sample_identity(sample: &[f64], target: f64, row: usize) -> u64 {
	const OFFSET: u64 = 14695981039346656037;
	const PRIME: u64 = 1099511628211;
	let hash = sample.iter().copied().chain(std::iter::once(target)).fold(OFFSET, |hash, value| (hash ^ value.to_bits()).wrapping_mul(PRIME));
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
	let metadata = fs::metadata(path).map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", path.display())))?;
	if metadata.is_file() {
		files.push(path.to_owned());
		return Ok(());
	}
	let mut children = fs::read_dir(path).map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?.collect::<std::io::Result<Vec<_>>>().map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
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
	let valid = |group: &[Table]| group.len() > 1 && targets.iter().all(|target| group.iter().filter(|table| target_column(table, target).is_some()).count() == 1 && group.iter().find(|table| target_column(table, target).is_some()).is_some_and(|table| table.rows.len() == 1));
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
		require(capture.iter().zip(&schemas).all(|(table, schema)| table.headers == schema.0 && table.rows.len() == schema.1), "capture table schemas differ")?;
	}
	let names = (0..schemas.len())
		.map(|index| {
			let name = &captures[0][index].name;
			if captures.iter().all(|capture| capture[index].name == *name) { name.clone() } else { format!("table{}", index + 1) }
		})
		.collect::<Vec<_>>();
	let children = names.iter().zip(&schemas).map(|(name, schema)| ChildTable { name: name.clone(), headers: schema.0.clone(), rows: schema.1 }).collect();
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
	let members = tables.iter().enumerate().filter_map(|(index, table)| targets.iter().all(|target| target_column(table, target).is_some()).then_some(index)).collect::<Vec<_>>();
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
			require(ignored || tables[index].headers.contains(header), format!("feature {header:?} is absent from partition {:?}", tables[index].name))?;
		}
	}
	let mut rows = Vec::new();
	for index in members {
		let positions = tables[index].headers.iter().map(|header| headers.iter().position(|value| value == header).unwrap()).collect::<Vec<_>>();
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
	let delimiter = [b',', b';', b'\t'].into_iter().max_by_key(|delimiter| first.iter().filter(|byte| *byte == delimiter).count()).unwrap_or(b',');
	let mut rows = records(bytes, delimiter)?;
	require(!rows.is_empty(), format!("dataset {} is empty", path.display()))?;
	let first = rows.remove(0);
	let headerless = first.iter().all(|value| value.parse::<f64>().is_ok());
	let headers = if headerless { (1..=first.len()).map(|column| format!("col{column}")).collect() } else { first.clone() };
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
	table.rows.iter().take(rows).filter_map(|row| row.get(column)).filter(|value| !value.is_empty()).cloned().collect::<BTreeSet<_>>().into_iter().collect()
}
fn infer_feature(table: &Table, column: usize, rows: usize) -> FeatureType {
	let values = table.rows.iter().take(rows).filter_map(|row| row.get(column)).filter(|value| !value.is_empty()).collect::<Vec<_>>();
	if !values.is_empty() && values.iter().all(|value| value.parse::<f64>().is_ok()) {
		return FeatureType::Numeric("f64");
	}
	let categories = categories(table, column, rows);
	if categories.len() < values.len() { FeatureType::Categorical(categories) } else { FeatureType::Text(values.iter().map(|value| value.len()).max().unwrap_or(0)) }
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
	let mut seed = env!("RECIPE_RANDOM_SEED").parse::<u64>().map_err(|error| RecipeError::new(format!("invalid random seed: {error}")))?;
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FloatFormat {
	family: &'static str,
	bits: u8,
	exponent: u8,
	mantissa: u8,
	llvm: &'static str,
}
const FP64: FloatFormat = FloatFormat { family: "fp", bits: 64, exponent: 11, mantissa: 52, llvm: "double" };
const FP32: FloatFormat = FloatFormat { family: "fp", bits: 32, exponent: 8, mantissa: 23, llvm: "float" };
const FP16: FloatFormat = FloatFormat { family: "fp", bits: 16, exponent: 5, mantissa: 10, llvm: "half" };
impl FloatFormat {
	fn bytes(self) -> usize {
		usize::from(self.bits.div_ceil(8))
	}
	fn native(self) -> Option<Self> { match (self.bits, self.exponent, self.mantissa) { (16, 5, 10) => Some(FP16), (32, 8, 23) => Some(FP32), (64, 11, 52) => Some(FP64), _ => None } }
	fn kernel(self) -> Option<usize> { [FP64, FP32, FP16].iter().position(|format| Some(*format) == self.native()) }
	fn label(self)->String{if self.family=="f"{format!("f({},{})",self.exponent,self.mantissa)}else{format!("{}({})",self.family,self.bits)}}
}
impl Train {
	pub const fn seed(mut self, value: usize) -> Self {
		self.seed = Some(value);
		self
	}
	pub const fn stop(mut self, value: f64) -> Self {
		self.stop = Some(value);
		self
	}
	pub const fn optimizer(self, _: Adamw) -> Self {
		self
	}
	pub const fn epochs(mut self, value: usize) -> Self {
		self.epochs = value;
		self
	}
	pub const fn lr(mut self, value: f64) -> Self {
		self.learning_rate = value;
		self
	}
	pub fn log<const N: usize>(mut self, metrics: [Metric; N]) -> Self {
		self.log_metrics = metrics.into();
		self
	}
	pub fn save(mut self, path: impl Into<String>) -> Self {
		self.save = Some(path.into());
		self
	}
	pub fn resume(mut self, path: impl Into<String>) -> Self {
		self.resume = Some(path.into());
		self
	}
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
		let precision = model.blocks.first().map(|block| block.format).unwrap_or(model.format);
		let support = |format: FloatFormat| format.kernel().is_some_and(|index| gpus.iter().all(|gpu| gpu.kernels[index].is_some()));
		let available = [FP16, FP32, FP64].into_iter().filter(|format| support(*format)).flat_map(|format| [format!("f({},{}) [{}]", format.exponent, format.mantissa, format.llvm), format!("fp({}) [{}]", format.bits, format.llvm)]).collect::<Vec<_>>().join(", ");
		require(model.blocks.iter().all(|block| block.format.native() == precision.native()), format!("mixed execution formats are unavailable on {}; available precision: {available}", gpus.iter().map(|gpu| gpu.name.as_str()).collect::<Vec<_>>().join(", ")))?;
		require(support(precision), format!("{} [{}] training is unavailable on {}; available precision: {available}", precision.label(), precision.llvm, gpus.iter().map(|gpu| gpu.name.as_str()).collect::<Vec<_>>().join(", ")))?;
		config.precision = precision;
		if let Some(seed) = self.seed {
			config.random_seed = seed;
		}
		require(model.downstream.is_none(), "model-valued loss requires .rat()")?;
		let prepared = prepare(data)?;
		let training_rows = ((prepared.rows as f64) * data.split).floor() as usize;
		require(training_rows != 0 && training_rows <= prepared.rows, "split must select training rows")?;
		let probability = model.loss.0 >= 4;
		let scale = probability.then(|| TargetScale::fit(&prepared.targets[..training_rows]));
		let target_values = prepared.targets.iter().map(|target| scale.map_or(*target, |scale| scale.encode(*target))).collect::<Vec<_>>();
		let (run, mut graph) = (RUN.fetch_add(1, Ordering::Relaxed) + 1, compile(model, prepared, training_rows, gpus[0], config)?);
		graph.state.training_rows = training_rows;
		if let Some(scale) = scale
			&& let Some(offset) = output_bias_offset(&graph)
		{
			let mean = target_values[..training_rows].iter().sum::<f64>() / training_rows as f64;
			graph.parameters[offset] = scale.logit(mean);
		}
		let mut stored = stored_graph(&graph, data, scale, precision);
		require(stored.graph.output.elements() == 1, "model output width must be one")?;
		if let Some(path) = &self.resume {
			bundle::restore(path, &prepared.schema, std::slice::from_mut(&mut stored), &prepared.identities)?;
			eprintln!("resumed: {path}");
		}
		stored.graph.state.trained_samples.extend_from_slice(&prepared.identities[..training_rows]);
		stored.graph.state.trained_samples.sort_unstable();
		stored.graph.state.trained_samples.dedup();
		let (samples, targets) = (&prepared.samples[..training_rows * prepared.features], &target_values[..training_rows]);
		let (mut tape, mut runtime) = (DeviceTape::new(&stored.graph, samples, targets, &gpus, config.precision)?, tile_runtime(&gpus, config)?);
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
			let dispatched = tape.advance().and_then(|_| tape.epoch(self.learning_rate, model.loss, tolerance, config, false));
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
		let predictions = raw_predictions.iter().map(|value| scale.map_or(*value, |scale| scale.decode(*value))).collect::<Vec<_>>();
		let r2 = if training_rows == prepared.rows {
			coefficient(&prepared.targets, &predictions)
		} else {
			let mut graph = stored.graph.clone();
			graph.parameters = tape.weights()?;
			let (start, validation_targets) = (training_rows * prepared.features, &target_values[training_rows..]);
			let mut validation = DeviceTape::new(&graph, &prepared.samples[start..], validation_targets, &gpus, config.precision)?;
			validation.forward().and_then(|_| validation.predictions()).map(|values| values.into_iter().map(|value| scale.map_or(value, |scale| scale.decode(value))).collect::<Vec<_>>()).map(|values| coefficient(&prepared.targets[training_rows..], &values))?
		};
		if training_rows < prepared.rows {
			eprintln!("Evaluation: r2 {r2:.4}");
		}
		self.finish_dispatch(Ok(()), &mut stored, &prepared.schema, &tape, Some(self.stop.is_some()))?;
		Ok(TrainingReport(initial_loss, final_loss, initial_predictions, predictions, r2, tape.tile()))
	}
	fn finish_dispatch<T>(&self, result: Result<T>, stored: &mut bundle::StoredGraph, schema: &str, tape: &DeviceTape, save: Option<bool>) -> Result<T> {
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
	fn print(&self, model: &Model, run: u64, epoch: usize, loss: f64, targets: &[f64], predictions: &[f64], seconds: f64, checkpoint: bool) {
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
	pub const fn initial_loss(&self) -> f64 {
		self.0
	}
	pub const fn final_loss(&self) -> f64 {
		self.1
	}
	pub fn initial_predictions(&self) -> &[f64] {
		&self.2
	}
	pub fn predictions(&self) -> &[f64] {
		&self.3
	}
	pub const fn r2(&self) -> f64 {
		self.4
	}
	pub const fn tile(&self) -> [u32; 3] {
		[self.5.m, self.5.n, self.5.k]
	}
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
	let mut result = values.map(|(prediction, target)| loss.value(*prediction, *target, threshold)).sum::<f64>() / targets.len() as f64;
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
		self.0.iter().find(|value| value.0 == name).map(|value| value.1).ok_or_else(|| RecipeError::new(format!("RAT value {name:?} is absent")))
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
		let mut child = Command::new(command).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().map_err(|error| RecipeError::new(format!("cannot start {command:?}: {error}")))?;
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
				let value = value.trim().parse::<f64>().map_err(|error| RecipeError::new(format!("RAT value {name:?} is invalid: {error}")))?;
				require(value.is_finite(), format!("RAT value {name:?} must be finite"))?;
				require(!values.iter().any(|item: &(String, f64)| item.0 == name), format!("RAT value {name:?} is duplicated"))?;
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
	pub fn proposal(&self) -> &[f64] {
		&self.proposal
	}
	pub fn prediction(&self) -> &[f64] {
		&self.prediction
	}
	pub fn measurement(&self) -> &[f64] {
		&self.measurement
	}
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
	data.routes.iter().map(|route| format!("{}->{}", route.inputs.join("|"), route.outputs.join("|"))).chain(std::iter::once(format!("target->{}", data.target.join("|")))).collect::<Vec<_>>().join("/")
}
fn append_model(graph: &mut Graph, model: &Model, features: usize, outputs: usize, rows: usize, gpu: &'static Gpu, config: Config, schema: &str) -> Result<(i32, (usize, usize))> {
	let prepared = Prepared { samples: vec![0.0; rows * features], targets: vec![0.0; rows], rows, features, schema: schema.to_owned(), sequence: None, norm_mean: Vec::new(), norm_scale: Vec::new(), identities: Vec::new() };
	let part = compile_output(model, &prepared, rows, gpu, config, outputs)?;
	let start = graph.parameters.len();
	let source = append_graph(graph, part)?;
	Ok((source, (start, graph.parameters.len())))
}
fn build<const N: usize>(models: &[Model; N], train: &Train, data: &Data, gpus: &[&'static Gpu], config: Config) -> Result<State> {
	require(N >= 2, "RAT requires an intermediate model and a predictor")?;
	require(data.routes.len() + 1 == N, "RAT requires one .r#in().out() pair per intermediate model")?;
	require(!data.target.is_empty(), "RAT requires .target()")?;
	let (rows, input_names) = (config.rat_batch, require(!data.routes[0].inputs.is_empty(), "RAT requires an initial input").map(|_| data.routes[0].inputs.clone())?);
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
		let (mut source, range) = append_model(&mut graph, &models[index], route.inputs.len(), route.outputs.len(), rows, gpus[0], config, &schema)?;
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
	let (_, range) = append_model(&mut graph, &models[N - 1], predictor_inputs.len(), data.target.len(), rows, gpus[0], config, &schema)?;
	ranges.push(range);
	let mut stored = bundle::StoredGraph { graph, precision: config.precision, inputs: input_names, outputs: data.target.clone(), norm_mean: Vec::new(), norm_scale: Vec::new(), target_min: 0.0, target_span: 0.0, bn_stats: Vec::new() };
	if let Some(path) = &train.resume {
		bundle::restore(path, &schema, std::slice::from_mut(&mut stored), &[])?;
		eprintln!("resumed: {path}");
	}
	let targets = vec![0.0; rows * stored.outputs.len()];
	let tape = DeviceTape::new(&stored.graph, &vec![0.0; rows * stored.inputs.len()], &targets, gpus, config.precision)?;
	Ok(State { stored, tape, proposals, models: ranges, proposal_names, targets, schema })
}
type TileCase = (usize, usize, [f64; 5], Tile, Tile);
struct TileRuntime {
	state: State,
	train: Train,
	predictor: Model,
	vram_unit: usize,
}
static TILE_RUNTIME: OnceLock<Result<Mutex<TileRuntime>>> = OnceLock::new();
impl TileRuntime {
	fn new(gpus: &[&'static Gpu], config: Config) -> Result<Self> {
		let predictor = recipe.model().layer(config.surrogate_width).tanh().layer(1).loss(mse);
		let proposer = recipe.model().layer(config.surrogate_width).tanh().layer(3).loss(&predictor);
		let models = [proposer, predictor];
		let data = recipe.data(Vec::<String>::new()).r#in(["dtype", "VRAM", "M", "N", "K"]).out(["m", "n", "k"]).target(["Mtime"]).norm(z_score);
		let train = recipe.train().epochs(config.surrogate_epochs).lr(config.surrogate_rate);
		Ok(Self { state: build(&models, &train, &data, gpus, config)?, train, predictor: models.into_iter().last().unwrap(), vram_unit: config.vram_unit })
	}
	fn forward(&mut self, cases: &[TileCase]) -> Result<()> {
		let samples = (0..self.state.tape.capacity).flat_map(|index| cases[index % cases.len()].2).collect::<Vec<_>>();
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
					let limit = shard.tape.gpu.proposal_limit(dimensions, shard.tape.precision)?;
					let minimum = Tile { m: shard.tape.tile.m.min(limit.m), n: shard.tape.tile.n.min(limit.n), k: shard.tape.tile.k.min(limit.k) };
					let work = dimensions[0] * f64::from(shard.tape.rows);
					let vram = if shard.tape.gpu.backend == Backend::Cpu { 0.0 } else { shard.tape.gpu.memory as f64 / self.vram_unit as f64 };
					cases.push((gpu, node, [f64::from(shard.tape.precision.bits), vram, work, dimensions[1], dimensions[2]], minimum, limit));
				}
			}
		}
		let mut tiles = tape.shards.iter().map(|shard| std::iter::repeat_n(shard.tape.tile, graph.nodes.len()).collect::<Vec<_>>()).collect::<Vec<_>>();
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
			let measurement = (0..self.state.tape.capacity).map(|index| elapsed[index % elapsed.len()]).collect::<Vec<_>>();
			train_rat(&mut self.state, &self.train, &self.predictor, &measurement, config)?;
		}
		Ok(())
	}
}
fn tile_runtime(gpus: &[&'static Gpu], config: Config) -> Result<std::sync::MutexGuard<'static, TileRuntime>> {
	TILE_RUNTIME.get_or_init(|| TileRuntime::new(gpus, config).map(Mutex::new)).as_ref().map_err(Clone::clone)?.lock().map_err(|_| RecipeError::new("tile runtime lock is poisoned"))
}
impl State {
	fn proposal(&self) -> Result<Vec<f64>> {
		let blocks = self.proposals.iter().map(|&(node, width)| Ok((self.tape.node_values(node, width)?, width))).collect::<Result<Vec<_>>>()?;
		let mut values = Vec::new();
		for row in 0..self.tape.capacity {
			for (block, width) in &blocks {
				values.extend_from_slice(&block[row * width..(row + 1) * width]);
			}
		}
		Ok(values)
	}
}
fn train_rat(state: &mut State, train: &Train, predictor: &Model, measurement: &[f64], config: Config) -> Result<Vec<f64>> {
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
	pub fn every(mut self, command: impl Into<String>) -> Self {
		self.command = Some(command.into());
		self
	}
	pub fn save(mut self, path: impl Into<String>) -> Self {
		self.train.save = Some(path.into());
		self
	}
	pub fn resume(mut self, path: impl Into<String>) -> Self {
		self.train.resume = Some(path.into());
		self
	}
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
			let limit = state.tape.proposal_limit(index, [frame.value("M")?, frame.value("N")?, frame.value("K")?])?;
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
