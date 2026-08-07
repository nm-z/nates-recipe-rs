//! Recipe executes one model graph after automatically probing a compiled discrete GPU backend.
//! Attention is three-projection scaled Q/K/V without an output projection.
#![allow(non_upper_case_globals)]
mod bundle {
	use super::*;
	use std::{collections::BTreeMap, io::Write as _, str::FromStr};
	#[derive(Clone)] pub(super) struct StoredGraph {
		pub graph: Graph, 		pub inputs: Vec<String>, 		pub outputs: Vec<String>, 	}
	#[derive(Default)] struct Builder { 		inputs: Vec<String>, 		outputs: Vec<String>,
		input: Option<Shape>, 		output: Option<Shape>, 		nodes: Vec<Node>, 		arguments: usize,
		parameters: Vec<f64>, 		frozen: Vec<u8>, 		programs: Vec<f64>, 	}
	impl Builder { 		fn finish(self) -> Result<StoredGraph> {
			let input = self.input.ok_or_else(|| RecipeError::new("model graph has no input shape"))?;
			let output = self.output.ok_or_else(|| RecipeError::new("model graph has no output shape"))?;
			require(!self.nodes.is_empty(), "model graph has no nodes")?;
			require(self.arguments == self.nodes.len(), "model graph node arguments are incomplete")?;
			require(self.parameters.len() == self.frozen.len(), "model graph frozen weights are incomplete")?;
			require(self.inputs.len() == input.elements(), "model graph input schema has the wrong width")?;
			require(self.outputs.len() == output.elements(), "model graph output schema has the wrong width")?;
			let source = self.nodes.len() as i32 - 1; 			Ok(StoredGraph {
				graph: Graph { nodes: self.nodes, parameters: self.parameters, frozen: self.frozen,
					programs: self.programs, input, output, source },
				inputs: self.inputs, 				outputs: self.outputs, 			}) 		} 	}
	fn values<T: FromStr>(text: &str, role: &str) -> Result<Vec<T>> where T::Err: fmt::Display {
		text.split_whitespace().map(|value| value.parse()
			.map_err(|error| RecipeError::new(format!("invalid {role}: {error}")))).collect() 	}
	fn primitive(value: i32) -> Result<Primitive> { 		match value {
			0 => Ok(Primitive::Contraction), 			2 => Ok(Primitive::Pool), 			3 => Ok(Primitive::Gather),
			4 => Ok(Primitive::Attention), 			5 => Ok(Primitive::Scan), 			6 => Ok(Primitive::Elementwise),
			7 => Ok(Primitive::Route), 			8 => Ok(Primitive::Normalize),
			_ => Err(RecipeError::new(format!("invalid primitive {value}"))), 		} 	}
	fn node(value: &str) -> Result<Node> { 		let value = values::<i32>(value, "node descriptor")?;
		require(value.len() == 11, "model graph node descriptor has the wrong width")?; 		Ok(Node {
			op: primitive(value[0])?, 			source: value[1], 			second: value[2],
			input: Shape { channels: value[3] as usize, length: value[4] as usize },
			output: Shape { channels: value[5] as usize, length: value[6] as usize },
			offset: value[7] as usize, 			parameters: value[8] as usize, 			argument: [0.0; 9],
			program_offset: value[9] as usize, 			program_count: value[10] as usize, 		}) 	}
	pub(super) fn load(path: &str) -> Result<(String, Vec<StoredGraph>)> {
		require(path.ends_with(".ogdl"), "model path requires .ogdl")?;
		let document = fs::read_to_string(path)
			.map_err(|error| RecipeError::new(format!("cannot read {path}: {error}")))?;
		let mut schema = String::new();
		let mut graphs = Vec::new();
		let mut current: Option<Builder> = None;
		for line in document.lines().map(str::trim) { 			if line == "recipe-model" { 				continue 			}
			if line == "graph" { 				if let Some(value) = current.take() { 					graphs.push(value.finish()?) 				}
				current = Some(Builder::default()); 				continue 			}
			let (kind, value) = line.split_once(' ').unwrap_or((line, "")); 			if kind == "schema" {
				schema = value.to_owned(); 				continue 			}
			let builder = current.as_mut().ok_or_else(|| RecipeError::new("model value precedes graph"))?;
			match kind { 				"in" => builder.inputs.push(value.to_owned()),
				"out" => builder.outputs.push(value.to_owned()), 				"shape" => {
					let shape = values::<usize>(value, "model shape")?;
					require(shape.len() == 4, "model graph shape has the wrong width")?;
					builder.input = Some(Shape { channels: shape[0], length: shape[1] });
					builder.output = Some(Shape { channels: shape[2], length: shape[3] }); 				}
				"node" => builder.nodes.push(node(value)?), 				"arguments" => {
					let argument = values::<f64>(value, "node argument")?;
					require(argument.len() == 9, "model graph node argument has the wrong width")?;
					builder.nodes.last_mut().ok_or_else(|| RecipeError::new("argument precedes node"))?
						.argument.copy_from_slice(&argument);
					builder.arguments += 1; 				}
				"programs" => builder.programs = values(value, "scalar program")?,
				"weights" => builder.parameters = values(value, "weight")?,
				"frozen" => builder.frozen = values(value, "frozen weight")?,
				"" => {}, 				_ => return Err(RecipeError::new(format!("invalid model value: {line}"))), 			} 		}
		if let Some(value) = current { 			graphs.push(value.finish()?) 		}
		require(!graphs.is_empty(), "model has no graphs")?; 		Ok((schema, graphs)) 	}
	fn join<T: ToString>(values: &[T]) -> String {
		values.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ") 	}
	pub(super) fn save(path: &str, schema: &str, graphs: &[StoredGraph]) -> Result<()> {
		require(path.ends_with(".ogdl"), "save requires an .ogdl model")?;
		require(!graphs.is_empty(), "model bundle has no graphs")?;
		let mut document = format!("recipe-model\n    schema {schema}\n"); 		for stored in graphs {
			document.push_str("    graph\n"); 			for name in &stored.inputs {
				document.push_str(&format!("        in {name}\n")) 			} 			for name in &stored.outputs {
				document.push_str(&format!("        out {name}\n")) 			}
			let graph = &stored.graph; 			document.push_str(&format!("        shape {} {} {} {}\n",
				graph.input.channels, graph.input.length, graph.output.channels, graph.output.length));
			for node in &graph.nodes { 				let descriptor = [node.op as i32, node.source, node.second,
					node.input.channels as i32, node.input.length as i32, node.output.channels as i32,
					node.output.length as i32, node.offset as i32, node.parameters as i32,
					node.program_offset as i32, node.program_count as i32];
				document.push_str(&format!("        node {}\n", join(&descriptor)));
				document.push_str(&format!("            arguments {}\n", join(&node.argument))); 			}
			document.push_str(&format!("        programs {}\n", join(&graph.programs)));
			document.push_str(&format!("        weights {}\n", join(&graph.parameters)));
			document.push_str(&format!("        frozen {}\n", join(&graph.frozen))); 		}
		fs::write(path, document).map_err(|error| RecipeError::new(format!("cannot write {path}: {error}")))?;
		eprintln!("saved: {path}"); 		Ok(()) 	}
	fn same_node(a: &Node, b: &Node) -> bool { 		a.op == b.op && a.source == b.source && a.second == b.second
			&& a.input == b.input && a.output == b.output && a.offset == b.offset && a.parameters == b.parameters
			&& a.argument.map(f64::to_bits) == b.argument.map(f64::to_bits) && a.program_offset == b.program_offset
			&& a.program_count == b.program_count 	}
	fn same_graph(a: &StoredGraph, b: &StoredGraph) -> bool { 		a.inputs == b.inputs && a.outputs == b.outputs
			&& a.graph.input == b.graph.input && a.graph.output == b.graph.output && a.graph.frozen == b.graph.frozen
			&& a.graph.programs == b.graph.programs && a.graph.parameters.len() == b.graph.parameters.len()
			&& a.graph.nodes.len() == b.graph.nodes.len()
			&& a.graph.nodes.iter().zip(&b.graph.nodes).all(|(a, b)| same_node(a, b)) 	}
	pub(super) fn restore(path: &str, schema: &str, graphs: &mut [StoredGraph]) -> Result<()> {
		if !fs::exists(path).map_err(|error| RecipeError::new(format!("cannot inspect {path}: {error}")))? {
			return save(path, schema, graphs) 		} 		let (stored_schema, stored) = load(path)?;
		let matches = stored_schema == schema && stored.len() == graphs.len()
			&& stored.iter().zip(graphs.iter()).all(|(a, b)| same_graph(a, b)); 		if matches {
			for (current, saved) in graphs.iter_mut().zip(stored) {
				current.graph.parameters = saved.graph.parameters 			} 			return Ok(()) 		}
		eprint!("mismatch: overwrite {path}? Y/n ");
		std::io::stderr().flush().map_err(|error| RecipeError::new(format!("cannot prompt: {error}")))?;
		let mut answer = String::new();
		std::io::stdin().read_line(&mut answer).map_err(|error| RecipeError::new(format!("cannot read answer: {error}")))?;
		require(answer.trim().is_empty() || answer.trim().eq_ignore_ascii_case("y"), "model mismatch not overwritten")?;
		save(path, schema, graphs) 	}
	pub(super) fn infer(path: &str, input: &[f64]) -> Result<Vec<f64>> { 		let (_, graphs) = load(path)?;
		let first = graphs.first().ok_or_else(|| RecipeError::new("model has no graph"))?;
		require(input.len() == first.inputs.len(), "model input has the wrong width")?;
		let mut values = first.inputs.iter().cloned().zip(input.iter().copied()).collect::<BTreeMap<_, _>>();
		let backend = device_backend()?;
		let config = Config::load()?;
		let mut result = Vec::new();
		for stored in graphs {
			let samples = stored.inputs.iter().map(|name| values.get(name).copied()
				.ok_or_else(|| RecipeError::new(format!("input {name:?} is absent")))).collect::<Result<Vec<_>>>()?;
			let mut tape = DeviceTape::new(&stored.graph, &samples, &[], backend, config.tile)?;
			tape.forward()?;
			result = tape.predictions()?;
			require(result.len() == stored.outputs.len(), "model output has the wrong width")?;
			for (name, value) in stored.outputs.iter().cloned().zip(result.iter().copied()) {
				values.insert(name, value); 			} 		} 		Ok(result) 	}
}
use std::{ 	collections::{BTreeMap, BTreeSet}, 	error::Error, 	ffi::c_void,
	fmt, fs, 	mem::{size_of, size_of_val}, 	path::{Path, PathBuf}, 	ptr, 	sync::{
		Mutex, OnceLock, 		atomic::{AtomicBool, AtomicU64, Ordering}, 	}, 	time::Instant, };
pub static recipe: Recipe = Recipe;
static RUN: AtomicU64 = AtomicU64::new(0);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
const SIGINT: i32 = 2;
const INTERRUPTED_EXIT: i32 = 128 + SIGINT;
static SIGNAL: OnceLock<usize> = OnceLock::new(); extern "C" fn interrupt(_: i32) {
	INTERRUPTED.store(true, Ordering::Release); } #[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeError(String); impl RecipeError { 	fn new(message: impl Into<String>) -> Self { 		Self(message.into())
	} } impl fmt::Display for RecipeError { 	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		formatter.write_str(&self.0) 	} } impl Error for RecipeError {}
pub type Result<T> = std::result::Result<T, RecipeError>;
type Ptr = *mut c_void; #[derive(Clone, Copy, Debug, PartialEq, Eq)] enum Backend { 	Amd, 	Nvidia, }
#[derive(Clone, Copy)] pub enum ArtifactSet { 	Auto, 	Amd, 	Nvidia, } pub struct Data { 	sources: Vec<String>,
	target: Vec<String>, 	exclusions: Vec<String>, 	routes: Vec<Route>, 	normalize: bool, 	split: f64,
	prepared: OnceLock<Result<Prepared>>, } #[derive(Clone)] struct Route { 	inputs: Vec<String>, 	outputs: Vec<String>, }
#[derive(Clone, Debug, PartialEq, Eq)] pub enum Residual { 	Layer(usize), 	Activation(Activation), }
pub const fn layer(width: usize) -> Residual { 	Residual::Layer(width) } #[derive(Clone, Debug, PartialEq, Eq)]
enum Operation {
	Layer(usize), 	Conv(usize, usize), 	Pool(usize),
	KMeans(usize), 	Knn(usize), 	Embed(usize, usize), 	Attention(usize), 	Rnn(usize), 	Gru(usize), 	Lstm(usize),
	Residual(Vec<Residual>), 	Moe(usize, Vec<Residual>), 	Svm(Vec<Activation>), 	Perceptron(usize), }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] #[repr(u8)] pub enum Activation {
	Linear, 	Cos, 	Exp, 	Log, 	Ln, 	Huber, 	Tan, 	Relu, 	Leak, 	Sigmoid, 	Tanh, 	Selu, 	Gelu, 	Silu,
	Elu, 	Prelu, } #[derive(Clone, Copy, Debug, PartialEq, Eq)] enum BlockNormalization { 	Batch, 	Layer, }
macro_rules! slots { ($(fn $name:ident = $value:ident),+ $(,)?) => {$(pub const fn $name() -> Residual {
	Residual::Activation(Activation::$value) })+}; } slots! {
fn linear = Linear, fn cos = Cos, fn exp = Exp, fn log = Log, fn ln = Ln, fn huber_atvn = Huber,
fn tan = Tan, fn relu = Relu, fn leak = Leak, fn sigmoid = Sigmoid, fn tanh = Tanh,
fn selu = Selu, fn gelu = Gelu, fn silu = Silu, fn elu = Elu, fn prelu = Prelu, }
#[derive(Clone, Debug, PartialEq, Eq)] struct Block { 	operation: Operation, 	activation: Activation,
	normalization: Option<BlockNormalization>, } pub struct Model {
	blocks: Vec<Block>, 	loss: LossFunction, 	downstream: Option<Vec<Block>>, }
pub trait ModelLoss { 	fn apply(self, model: &mut Model); } impl ModelLoss for LossFunction {
	fn apply(self, model: &mut Model) { 		model.loss = self;
		model.downstream = None; 	} } impl ModelLoss for &Model { 	fn apply(self, model: &mut Model) {
		model.downstream = Some(self.blocks.clone()); 	} }
macro_rules! operation_methods { ($(fn $method:ident($($argument:ident: $kind:ty),*) = $operation:expr;)+) => {
$(pub fn $method(self, $($argument: $kind),*) -> Self { self.push($operation) })+ }; } impl Model {
	fn push(mut self, operation: Operation) -> Self {
		self.blocks.push(Block { operation, activation: Activation::Linear, normalization: None }); 		self 	}
	fn activate(mut self, activation: Activation) -> Self {
		let block = self.blocks.last_mut().unwrap_or_else(|| panic!("activation requires a preceding block"));
		if block.normalization.is_some() { 			panic!("activation must precede normalization"); 		}
		block.activation = activation; 		self 	} 	operation_methods! {
	fn layer(width: usize) = Operation::Layer(width);
	fn conv(filters: usize, kernel: usize) = Operation::Conv(filters, kernel);
	fn pool(size: usize) = Operation::Pool(size);
	fn kmeans(clusters: usize) = Operation::KMeans(clusters);
	fn knn(neighbors: usize) = Operation::Knn(neighbors);
	fn embed(dimensions: usize, vocabulary: usize) = Operation::Embed(dimensions, vocabulary);
	fn attn(heads: usize) = Operation::Attention(heads);
	fn rnn(width: usize) = Operation::Rnn(width);
	fn gru(width: usize) = Operation::Gru(width);
	fn lstm(width: usize) = Operation::Lstm(width);
	fn perc(width: usize) = Operation::Perceptron(width); }
	pub fn residual<const N: usize>(self, parts: [Residual; N]) -> Self { 		self.push(Operation::Residual(parts.into())) 	}
	pub fn moe<const N: usize>(self, top_k: usize, experts: [Residual; N]) -> Self {
		self.push(Operation::Moe(top_k, experts.into())) 	}
	pub fn svm<const N: usize>(self, choices: [fn() -> Residual; N]) -> Self {
		let choices = choices.into_iter().map(|choice| match choice() { 			Residual::Activation(value) => value,
			Residual::Layer(_) => panic!("SVM choices must be activations"), 		}).collect();
		self.push(Operation::Svm(choices)) 	}
	pub fn norm(mut self, normalization: Normalization) -> Self {
		let block = self.blocks.last_mut().unwrap_or_else(|| panic!("normalization requires a preceding block"));
		block.normalization = Some(if normalization as usize == batch as usize { 			BlockNormalization::Batch 		} else {
			BlockNormalization::Layer 		}); 		self 	} 	pub fn loss(mut self, loss: impl ModelLoss) -> Self {
		loss.apply(&mut self); 		self 	} 	fn description(&self, metrics: &[Metric]) -> String {
		let operation = metrics.iter().any(|metric| metric.0 == 5);
		let activation = metrics.iter().any(|metric| metric.0 == 6);
		let normalization = metrics.iter().any(|metric| metric.0 == 7); 		self.blocks 			.iter() 			.filter_map(|block| {
				let mut names = Vec::new(); 				if operation {
					names.push(block.operation.name()); 				} 				if activation && block.activation != Activation::Linear {
					names.push(block.activation.name()); 				} 				if normalization { 					block.normalization
						.map(BlockNormalization::name) 						.into_iter() 						.for_each(|name| names.push(name)); 				}
				(!names.is_empty()).then(|| names.join(".")) 			}) 			.collect::<Vec<_>>() 			.join("/") 	} } impl Operation {
	const fn name(&self) -> &'static str { 		match self { 			Self::Layer(_) => "layer", 			Self::Conv(..) => "conv",
			Self::Pool(_) => "pool", 			Self::KMeans(_) => "kmeans", 			Self::Knn(_) => "knn", 			Self::Embed(..) => "embed",
			Self::Attention(_) => "attn", 			Self::Rnn(_) => "rnn", 			Self::Gru(_) => "gru", 			Self::Lstm(_) => "lstm",
			Self::Residual(_) => "residual", 			Self::Moe(..) => "moe", 			Self::Svm(_) => "svm",
			Self::Perceptron(_) => "perc", 		} 	} } impl Activation {
	const fn name(self) -> &'static str { 		match self { 			Self::Linear => "linear", 			Self::Cos => "cos",
			Self::Exp => "exp", 			Self::Log => "log", 			Self::Ln => "ln", 			Self::Huber => "huber", 			Self::Tan => "tan",
			Self::Relu => "relu", 			Self::Leak => "leak", 			Self::Sigmoid => "sigmoid", 			Self::Tanh => "tanh",
			Self::Selu => "selu", 			Self::Gelu => "gelu", 			Self::Silu => "silu", 			Self::Elu => "elu",
			Self::Prelu => "prelu", 		} 	} } impl BlockNormalization { 	const fn name(self) -> &'static str { 		match self {
			Self::Batch => "bnorm", 			Self::Layer => "lnorm", 		} 	} }
macro_rules! activations { ($(fn $method:ident = $activation:ident;)+) => {$(impl Model { pub fn $method(self) -> Self {
self.activate(Activation::$activation) } })+}; } activations! {
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
pub struct Adamw; #[derive(Clone, Copy)]
pub struct LossFunction(u8); #[derive(Clone, Copy)]
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
pub const batch: Normalization = batch_marker; const fn batch_marker(_: usize) -> Residual {
	Residual::Activation(Activation::Relu) }
impl LossFunction { 	const fn name(self) -> &'static str { 		match self.0 { 			0 => "mse", 			1 => "rmse",
			2 => "huber", 			3 => "mae", 			4 => "bce", 			5 => "ce", 			6 => "focal", 			_ => unreachable!(), 		} 	}
	fn value(self, prediction: f64, target: f64, threshold: f64) -> f64 { 		let difference = prediction - target;
		let probability = logistic(prediction).clamp(f64::EPSILON, 1.0 - f64::EPSILON); 		match self.0 {
			0 | 1 => difference * difference, 			2 => { 				let absolute = difference.abs(); 				if absolute <= threshold {
					0.5 * difference * difference 				} else { 					threshold * (absolute - 0.5 * threshold) 				} 			}
			3 => difference.abs(), 			4 | 5 => -target * probability.ln() - (1.0 - target) * (1.0 - probability).ln(), 			6 => {
				let correct = if target >= 0.5 { probability } else { 1.0 - probability };
				-(1.0 - correct).powi(2) * correct.ln() 			} 			_ => f64::NAN, 		} 	} } impl Recipe {
	pub fn data(&self, sources: impl IntoDataSources) -> Data { 		Data { 			sources: sources.into_data_sources(),
			target: Vec::new(), 			exclusions: Vec::new(), 			routes: Vec::new(), 			normalize: false,
			split: 1.0, 			prepared: OnceLock::new(), 		}
	} 	pub fn model(&self) -> Model { 		Model { blocks: Vec::new(), loss: mse, downstream: None } 	}
	pub const fn train(&self) -> Train {
		Train { epochs: 1, learning_rate: 0.001, log_metrics: Vec::new(), stop: None, resume: None, save: None } 	}
	pub fn export(&self, source: impl AsRef<Path>, selection: ArtifactSet) -> Result<Vec<PathBuf>> {
		let source = source.as_ref(); 		require( 			source.extension().and_then(|value| value.to_str()) == Some("rs"),
			"export requires a Rust source", 		)?; 		fs::metadata(source)
			.map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", source.display())))?;
		let backends = match selection { 			ArtifactSet::Auto => {
				[Backend::Amd, Backend::Nvidia].into_iter().filter(|backend| gpu(*backend).is_ok()).collect() 			}
			ArtifactSet::Amd => vec![Backend::Amd], 			ArtifactSet::Nvidia => vec![Backend::Nvidia], 		};
		let mut outputs = Vec::new(); 		for backend in backends { 			let artifacts = match backend { 				Backend::Amd => vec![
					("hsaco", option_env!("RECIPE_HSA_CODE_OBJECT")), 					("amd.s", option_env!("RECIPE_HSA_ASSEMBLY")), 				],
				Backend::Nvidia => vec![ 					("ptx", option_env!("RECIPE_NV_PTX")),
					("cubin", option_env!("RECIPE_NV_MODULE")), 					("nvidia.sass", option_env!("RECIPE_NV_SASS")), 				], 			};
			for (extension, compiled) in artifacts { 				let compiled = compiled
					.ok_or_else(|| RecipeError::new(format!("{backend:?} artifacts were not compiled")))?;
				let output = source.with_file_name(format!("recipe.{extension}")); 				fs::copy(compiled, &output).map_err(|error| {
					RecipeError::new(format!("cannot export {}: {error}", output.display())) 				})?;
				eprintln!("exported: {}", output.display());
				outputs.push(output); 			} 		} 		Ok(outputs) 	} }
impl Recipe { 	pub fn infer(&self, path: impl AsRef<Path>, input: &[f64]) -> Vec<f64> {
		let path = path.as_ref().to_string_lossy();
		bundle::infer(&path, input).unwrap_or_else(|error| panic!("{error}")) 	} }
#[derive(Clone, Copy, Debug, PartialEq, Eq)] struct Shape {
	channels: usize, 	length: usize, } impl Shape { 	fn elements(self) -> usize { 		self.channels * self.length 	} }
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)] enum Primitive { 	Contraction = 0, 	Pool = 2, 	Gather = 3, 	Attention = 4, 	Scan = 5, 	Elementwise = 6,
	Route = 7, 	Normalize = 8, } #[derive(Clone, Copy)] #[repr(i32)] enum ScalarOpcode {
	Add, 	Constant, 	Parameter, 	Subtract,
	Multiply, 	Divide, 	Absolute, 	Exp, 	Log, 	Sin = 10, 	Cos, 	Tanh, 	Greater, } struct ScalarProgram(Vec<f64>);
impl ScalarProgram { 	fn op(&mut self, opcode: ScalarOpcode, left: f64, right: f64) -> f64 {
		let result = (self.0.len() / 3) as f64;
		self.0.extend([opcode as i32 as f64, left, right]); 		result 	} 	fn constant(&mut self, value: f64) -> f64 {
		self.op(ScalarOpcode::Constant, value, 0.0) 	} 	fn choose(&mut self, condition: f64, yes: f64, no: f64) -> f64 {
		let one = self.constant(1.0);
		let inverse = self.op(ScalarOpcode::Subtract, one, condition);
		let selected = self.op(ScalarOpcode::Multiply, condition, yes);
		let alternative = self.op(ScalarOpcode::Multiply, inverse, no); 		self.op(ScalarOpcode::Add, selected, alternative) 	}
	fn unary(&mut self, opcode: ScalarOpcode, value: f64) -> f64 { 		self.op(opcode, value, 0.0) 	} } #[derive(Clone)]
struct Node { 	op: Primitive, 	source: i32, 	second: i32, 	input: Shape, 	output: Shape, 	offset: usize,
	parameters: usize, 	argument: [f64; 9], 	program_offset: usize, 	program_count: usize, }
#[derive(Clone)] struct Graph {
	nodes: Vec<Node>, 	parameters: Vec<f64>, 	frozen: Vec<u8>, 	programs: Vec<f64>, 	input: Shape, 	output: Shape,
	source: i32, }
fn compile(model: &Model, data: &Prepared, rows: usize, backend: Backend, config: Config) -> Result<Graph> {
	compile_graph(model, data, rows, backend, config, None) }
fn compile_output(
	model: &Model, data: &Prepared, rows: usize, backend: Backend, config: Config, output: usize,
) -> Result<Graph> {
	compile_graph(model, data, rows, backend, config, Some(output)) }
fn compile_graph(
	model: &Model, data: &Prepared, rows: usize, backend: Backend, config: Config, expected: Option<usize>,
) -> Result<Graph> {
	require(!model.blocks.is_empty(), "model must contain a block")?; 	let sequential =
		matches!(model.blocks[0].operation, Operation::Conv(..) | Operation::Pool(..) | Operation::Embed(..));
	let shape = if sequential { 		Shape { channels: 1, length: data.features } 	} else {
		Shape { channels: data.features, length: 1 } 	}; 	let mut graph = Graph { 		nodes: Vec::new(),
		parameters: Vec::new(), 		frozen: Vec::new(), 		programs: Vec::new(), 		input: shape, 		output: shape,
		source: -1, 	};
	for block in &model.blocks { 		lower_block(&mut graph, block, data, rows, backend, config)?; 	}
	if let Some(expected) = expected {
		require(graph.output.elements() == expected, "model output width does not match .out()")?;
	} else if graph.output.elements() != 1 { 		let length = graph.output.length;
		lower_conv(&mut graph, 1, length)?; 	}
	initialize_graph(&mut graph, config); 	Ok(graph) }
#[derive(Clone, Copy)] struct Field { 	source: i32, 	stride: usize, 	index: usize, }
fn field(fields: &[(String, Field)], name: &str) -> Result<Field> {
	fields.iter().find(|value| value.0 == name).map(|value| value.1)
		.ok_or_else(|| RecipeError::new(format!("RAT value {name:?} is not yet available"))) }
fn route_graph(graph: &mut Graph, names: &[String], fields: &[(String, Field)]) -> Result<()> {
	let offset = graph.programs.len();
	for name in names { 		let value = field(fields, name)?;
		graph.programs.extend([f64::from(value.source), value.stride as f64, value.index as f64]); 	}
	let output = Shape { channels: names.len(), length: 1 };
	let argument = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
	push_node(graph, Primitive::Route, output, 0, argument, -2)?;
	let node = graph.nodes.last_mut().ok_or_else(|| RecipeError::new("RAT route node is absent"))?;
	node.program_offset = offset;
	node.program_count = names.len(); 	Ok(()) }
fn append_graph(graph: &mut Graph, mut part: Graph) -> Result<i32> {
	let source = graph.source;
	let node_base = narrow(graph.nodes.len(), "RAT graph nodes")?;
	let weight_base = graph.parameters.len();
	let program_base = graph.programs.len();
	for node in &mut part.nodes { 		node.source = if node.source < 0 { source } else { node.source + node_base };
		if node.second >= 0 { 			node.second += node_base 		}
		node.offset = checked_add(node.offset, weight_base, "RAT weight offset")?;
		if node.program_count != 0 {
			node.program_offset = checked_add(node.program_offset, program_base, "RAT program offset")?; 		} 	}
	graph.parameters.extend(part.parameters);
	graph.frozen.extend(part.frozen);
	graph.programs.extend(part.programs);
	graph.nodes.extend(part.nodes);
	graph.output = part.output;
	graph.source = narrow(graph.nodes.len(), "RAT graph nodes")? - 1; 	Ok(graph.source) }
fn lower_block( 	graph: &mut Graph, 	block: &Block,
	data: &Prepared, 	rows: usize, 	backend: Backend, 	config: Config, ) -> Result<()> {
	let skip = graph.source; 	match &block.operation {
		Operation::Layer(width) | Operation::Perceptron(width) => lower_project(graph, *width)?,
		Operation::Conv(f, k) => lower_conv(graph, *f, *k)?, 		Operation::Pool(size) => lower_pool(graph, *size)?,
		Operation::Embed(dimensions, vocabulary) => lower_gather(graph, *dimensions, *vocabulary)?,
		Operation::Attention(heads) => lower_attention(graph, *heads)?,
		Operation::Rnn(width) => lower_scan(graph, *width, 1)?, 		Operation::Gru(width) => lower_scan(graph, *width, 3)?,
		Operation::Lstm(width) => lower_scan(graph, *width, 4)?,
		Operation::Residual(parts) => lower_residual(graph, parts, skip, config)?,
		Operation::Moe(top_k, experts) => lower_moe(graph, *top_k, experts, config)?,
		Operation::Svm(choices) => lower_svm(graph, choices, config)?,
		Operation::KMeans(_) | Operation::Knn(_) => { 			initialize_graph(graph, config);
			lower_estimator(graph, &block.operation, data, rows, backend, config)? 		} 	}
	if block.activation != Activation::Linear { 		lower_activation(graph, block.activation, config)?; 	}
	if let Some(normalization) = block.normalization {
		let epsilon = number("normalization epsilon", env!("RECIPE_NORMALIZATION_EPSILON"))?; 		push_node( 			graph,
			Primitive::Normalize, 			graph.output, 			0, 			arguments(normalization as u8 as f64, epsilon), 			-2, 		)?; 	}
	let elements = checked_mul(rows, graph.output.elements(), "node batch")?;
	narrow(elements, "GPU node batch")?; 	Ok(()) } fn push_node( 	graph: &mut Graph, 	op: Primitive, 	output: Shape,
	parameters: usize, 	argument: [f64; 9], 	second: i32, ) -> Result<()> {
	let source = graph.source;
	let offset = graph.parameters.len();
	graph.parameters.resize(checked_add(offset, parameters, "model parameters")?, 0.0);
	graph.frozen.resize(graph.parameters.len(), 0); 	graph.nodes.push(Node { 		op, 		source, 		second,
		input: graph.output, 		output, 		offset, 		parameters, 		argument, 		program_offset: 0, 		program_count: 0, 	});
	graph.output = output;
	graph.source = graph.nodes.len() as i32 - 1; 	Ok(()) }
fn push_program(graph: &mut Graph, second: i32, initial: &[f64], program: ScalarProgram) -> Result<()> {
	let program_offset = graph.programs.len();
	let program_count = program.0.len() / 3;
	graph.programs.extend(program.0);
	let arguments = arguments(0.0, 0.0);
	push_node(graph, Primitive::Elementwise, graph.output, initial.len(), arguments, second)?;
	let node = graph.nodes.last_mut().ok_or_else(|| RecipeError::new("scalar program node is absent"))?;
	graph.parameters[node.offset..node.offset + initial.len()].copy_from_slice(initial);
	node.program_offset = program_offset;
	node.program_count = program_count; 	Ok(()) }
fn lower_activation(graph: &mut Graph, activation: Activation, config: Config) -> Result<()> {
	let mut program = ScalarProgram(Vec::new());
	let x = -1.0;
	let zero = program.constant(0.0);
	let one = program.constant(1.0);
	let positive = program.op(ScalarOpcode::Greater, x, zero);
	let constant = |program: &mut ScalarProgram, value| program.constant(value); 	let result = match activation {
		Activation::Cos => program.unary(ScalarOpcode::Cos, x), 		Activation::Exp => program.unary(ScalarOpcode::Exp, x),
		Activation::Log | Activation::Ln => { 			let absolute = program.unary(ScalarOpcode::Absolute, x);
			let shifted = program.op(ScalarOpcode::Add, one, absolute);
			let magnitude = program.unary(ScalarOpcode::Log, shifted);
			let negative = program.op(ScalarOpcode::Subtract, zero, magnitude);
			let signed = program.choose(positive, magnitude, negative); 			if activation == Activation::Log {
				let base = constant(&mut program, std::f64::consts::LN_10); 				program.op(ScalarOpcode::Divide, signed, base)
			} else { 				signed 			} 		} 		Activation::Huber => {
			let threshold = constant(&mut program, config.activation[7]);
			let absolute = program.unary(ScalarOpcode::Absolute, x);
			let large = program.op(ScalarOpcode::Greater, absolute, threshold);
			let square = program.op(ScalarOpcode::Multiply, x, x);
			let half = constant(&mut program, 0.5);
			let small = program.op(ScalarOpcode::Multiply, half, square);
			let half_threshold = program.op(ScalarOpcode::Multiply, half, threshold);
			let excess = program.op(ScalarOpcode::Subtract, absolute, half_threshold);
			let tail = program.op(ScalarOpcode::Multiply, threshold, excess); 			program.choose(large, tail, small) 		}
		Activation::Tan => { 			let sine = program.unary(ScalarOpcode::Sin, x);
			let cosine = program.unary(ScalarOpcode::Cos, x); 			program.op(ScalarOpcode::Divide, sine, cosine) 		}
		Activation::Relu => program.op(ScalarOpcode::Multiply, positive, x),
		Activation::Leak | Activation::Elu | Activation::Selu | Activation::Prelu => { 			let negative = match activation {
				Activation::Leak => { 					let slope = constant(&mut program, config.activation[0]);
					program.op(ScalarOpcode::Multiply, slope, x) 				} 				Activation::Prelu => {
					let slope = program.op(ScalarOpcode::Parameter, 0.0, 0.0); 					program.op(ScalarOpcode::Multiply, slope, x) 				}
				_ => { 					let exponential = program.unary(ScalarOpcode::Exp, x);
					let shifted = program.op(ScalarOpcode::Subtract, exponential, one); 					let alpha = constant( 						&mut program,
						config.activation[usize::from(activation == Activation::Selu) + 2], 					);
					program.op(ScalarOpcode::Multiply, alpha, shifted) 				} 			};
			let selected = program.choose(positive, x, negative); 			if activation == Activation::Selu {
				let scale = constant(&mut program, config.activation[4]); 				program.op(ScalarOpcode::Multiply, scale, selected)
			} else { 				selected 			} 		} 		Activation::Sigmoid | Activation::Silu => {
			let negative = program.op(ScalarOpcode::Subtract, zero, x);
			let exponential = program.unary(ScalarOpcode::Exp, negative);
			let denominator = program.op(ScalarOpcode::Add, one, exponential);
			let sigmoid = program.op(ScalarOpcode::Divide, one, denominator);
			if activation == Activation::Silu { program.op(ScalarOpcode::Multiply, x, sigmoid) } else { sigmoid } 		}
		Activation::Tanh => program.unary(ScalarOpcode::Tanh, x), 		Activation::Gelu => {
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
			let half_x = program.op(ScalarOpcode::Multiply, half, x); 			program.op(ScalarOpcode::Multiply, half_x, shifted) 		}
		Activation::Linear => unreachable!(), 	};
	let initial = if activation == Activation::Prelu { &config.activation[1..2] } else { &[] };
	debug_assert_eq!(result as usize + 1, program.0.len() / 3); 	push_program(graph, -2, initial, program) }
fn lower_project(graph: &mut Graph, channels: usize) -> Result<()> {
	require(channels != 0, "layer width must be positive")?;
	let parameters = checked_mul(graph.output.channels, channels, "projection parameters")?;
	let output = Shape { channels, length: graph.output.length };
	push_node(graph, Primitive::Contraction, output, parameters, [0.0; 9], -2) }
fn lower_conv(graph: &mut Graph, filters: usize, kernel: usize) -> Result<()> {
	require(filters != 0 && kernel != 0, "convolution dimensions must be positive")?;
	require(kernel <= graph.output.length, "convolution kernel exceeds sequence length")?;
	let parameters = checked_mul(filters, checked_mul(graph.output.channels, kernel, "convolution window")?, "conv")?;
	let output = Shape { channels: filters, length: graph.output.length - kernel + 1 };
	push_node(graph, Primitive::Contraction, output, parameters, arguments(kernel as f64, 0.0), -2) }
fn lower_pool(graph: &mut Graph, size: usize) -> Result<()> { 	require(size != 0, "pool window must be positive")?;
	let output = Shape { channels: graph.output.channels, length: graph.output.length.div_ceil(size) };
	push_node(graph, Primitive::Pool, output, 0, arguments(size as f64, 0.0), -2) }
fn lower_gather(graph: &mut Graph, dimensions: usize, vocabulary: usize) -> Result<()> {
	require(dimensions != 0 && vocabulary != 0, "embedding dimensions must be positive")?;
	let parameters = checked_mul(dimensions, vocabulary, "embedding table")?;
	let output = Shape { channels: dimensions, length: graph.output.elements() };
	push_node(graph, Primitive::Gather, output, parameters, arguments(vocabulary as f64, 0.0), -2) }
fn lower_attention(graph: &mut Graph, heads: usize) -> Result<()> {
	require(heads != 0 && graph.output.channels % heads == 0, "attention head partition is invalid")?;
	let matrix = checked_mul(graph.output.channels, graph.output.channels, "attention matrix")?; 	push_node( 		graph,
		Primitive::Attention, 		graph.output, 		checked_mul(3, matrix, "QKV")?, 		arguments(heads as f64, 0.0), 		-2, 	) }
const ACTIVATIONS: [Activation; 16] = [
	Activation::Linear, Activation::Cos, Activation::Exp, Activation::Log, Activation::Ln, Activation::Huber,
	Activation::Tan, Activation::Relu, Activation::Leak, Activation::Sigmoid, Activation::Tanh, Activation::Selu,
	Activation::Gelu, Activation::Silu, Activation::Elu, Activation::Prelu,
];
fn reset(graph: &mut Graph, source: i32, shape: Shape) {
	graph.source = source;
	graph.output = shape;
}
fn program(
	graph: &mut Graph, first: i32, second: i32, shape: Shape, initial: &[f64], program: ScalarProgram,
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
	graph: &mut Graph, branches: &[i32], scores: &[i32], shape: Shape, top_k: usize, config: Config,
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
fn lower_svm(graph: &mut Graph, choices: &[Activation], config: Config) -> Result<()> {
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
fn lower_scan(graph: &mut Graph, channels: usize, gates: usize) -> Result<()> {
	require(channels != 0, "recurrent width must be positive")?;
	let input = checked_mul(graph.output.channels, channels, "scan input matrix")?;
	let state = checked_mul(channels, channels, "scan state matrix")?;
	let stride = checked_add(checked_add(input, state, "scan gate")?, channels, "scan bias")?;
	let output = Shape { channels, length: graph.output.length }; 	push_node( 		graph, 		Primitive::Scan, 		output,
		checked_mul(gates, stride, "scan parameters")?, 		arguments(gates as f64, 0.0), 		-2, 	) }
fn lower_residual(graph: &mut Graph, parts: &[Residual], skip: i32, config: Config) -> Result<()> {
	let shape = graph.output;
	require(!parts.is_empty(), "residual branch must contain an operation")?; 	for part in parts { 		match part {
			Residual::Layer(width) => lower_project(graph, *width)?,
			Residual::Activation(activation) => lower_activation(graph, *activation, config)?, 		} 	} 	require(
		graph.output.channels == shape.channels && graph.output.length == shape.length, 		"residual shape mismatch", 	)?;
	let mut program = ScalarProgram(Vec::new());
	program.op(ScalarOpcode::Add, -1.0, -2.0); 	push_program(graph, skip, &[], program) }
fn lower_estimator(
	graph: &mut Graph,
	operation: &Operation,
	data: &Prepared,
	rows: usize,
	backend: Backend,
	config: Config,
) -> Result<()> {
	let input = graph.output;
	let source = graph.source;
	let inputs = graph_inputs(graph, &data.samples, &data.targets, rows, backend, config.tile)?;
	match operation {
		Operation::KMeans(hidden) => {
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
			};
			let teacher = estimator_predict(operation, &paired, rows, config, true)?;
			let surrogate = fit_surrogate(input, &inputs, &teacher, *hidden, backend, config)?;
			let first = checked_mul(*hidden, input.elements(), "surrogate input")?;
			require(surrogate.len() == first + hidden, "surrogate state has the wrong size")?;
			push_frozen(
				graph,
				Primitive::Contraction,
				input,
				Shape { channels: *hidden, length: 1 },
				&surrogate[..first],
				arguments(input.length as f64, 0.0),
				source,
			)?;
			lower_activation(graph, Activation::Tanh, config)?;
			let source = graph.source;
			push_frozen(
				graph,
				Primitive::Contraction,
				Shape { channels: *hidden, length: 1 },
				Shape { channels: 1, length: 1 },
				&surrogate[first..],
				[0.0; 9],
				source,
			)
		}
		Operation::Knn(top_k) => {
			require(*top_k != 0 && *top_k <= rows, "knn neighbor count is invalid")?;
			let scalar = Shape { channels: 1, length: 1 };
			let mut candidates = Vec::with_capacity(rows);
			let mut scores = Vec::with_capacity(rows);
			for (row, sample) in inputs.chunks_exact(input.elements()).enumerate() {
				reset(graph, source, input);
				let weights = sample.iter().map(|value| 2.0 * value).collect::<Vec<_>>();
				push_frozen(
					graph,
					Primitive::Contraction,
					input,
					scalar,
					&weights,
					arguments(input.length as f64, 0.0),
					source,
				)?;
				let projection = graph.source;
				let bias_value = -sample.iter().map(|value| value * value).sum::<f64>();
				let bias = constant(graph, projection, scalar, bias_value)?;
				scores.push(binary(graph, projection, bias, scalar, ScalarOpcode::Add)?);
				candidates.push(constant(graph, projection, scalar, data.targets[row])?);
			}
			select(graph, &candidates, &scores, scalar, *top_k, config)
		}
		_ => Err(RecipeError::new("operation is not an estimator")),
	}
}
fn initialize_graph(graph: &mut Graph, config: Config) {
	for (weight, frozen) in graph.parameters.iter_mut().zip(&graph.frozen) { 		if *frozen == 0 {
			*weight = config.initial; 		} 	} } fn push_frozen( 	graph: &mut Graph, 	op: Primitive, 	input: Shape, 	output: Shape,
	values: &[f64], 	argument: [f64; 9], 	source: i32, ) -> Result<()> {
	let offset = graph.parameters.len();
	graph.parameters.extend_from_slice(values);
	graph.frozen.resize(graph.parameters.len(), 1); 	graph.nodes.push(Node { 		op, 		source, 		second: -2, 		input,
		output, 		offset, 		parameters: 0, 		argument, 		program_offset: 0, 		program_count: 0, 	});
	graph.output = output;
	graph.source = graph.nodes.len() as i32 - 1; 	Ok(()) }
fn arguments(first: f64, second: f64) -> [f64; 9] { 	[first, second, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] }
fn checked_add(left: usize, right: usize, role: &str) -> Result<usize> {
	left.checked_add(right).ok_or_else(|| RecipeError::new(format!("{role} overflows"))) }
fn checked_mul(left: usize, right: usize, role: &str) -> Result<usize> {
	left.checked_mul(right).ok_or_else(|| RecipeError::new(format!("{role} overflows"))) }
fn require(condition: bool, message: impl Into<String>) -> Result<()> {
	condition.then_some(()).ok_or_else(|| RecipeError::new(message)) } fn logistic(value: f64) -> f64 {
	1.0 / (1.0 + (-value).exp()) } #[derive(Clone, Copy)] struct Tile { 	m: u32, 	n: u32, 	k: u32, }
impl Tile { 	fn load(maximum: bool) -> Result<Self> { 		let value = |name, text| {
			Ok(narrow(natural(name, text)?, name)? as u32) 		}; 		if maximum { 			Ok(Self {
				m: value("maximum contraction M tile", env!("RECIPE_TILE_M_MAX"))?,
				n: value("maximum contraction N tile", env!("RECIPE_TILE_N_MAX"))?,
				k: value("maximum contraction K tile", env!("RECIPE_TILE_K_MAX"))?, 			}) 		} else { 			Ok(Self {
				m: value("contraction M tile", env!("RECIPE_TILE_M"))?,
				n: value("contraction N tile", env!("RECIPE_TILE_N"))?,
				k: value("contraction K tile", env!("RECIPE_TILE_K"))?, 			}) 		} 	}
	fn proposed(values: &mut [f64], limit: Self) -> Result<Self> {
		require(values.len() >= 3, "RAT requires M, N, and K proposal outputs")?;
		let dimension = |value: f64, maximum: u32| -> Result<u32> {
			require(value.is_finite(), "RAT tile proposal must be finite")?;
			Ok(1 + (logistic(value) * f64::from(maximum - 1)).floor() as u32) 		};
		let tile = Self { m: dimension(values[0], limit.m)?, n: dimension(values[1], limit.n)?,
			k: dimension(values[2], limit.k)? };
		values[..3].copy_from_slice(&[f64::from(tile.m), f64::from(tile.n), f64::from(tile.k)]); 		Ok(tile) 	} }
#[derive(Clone, Copy)] struct Config { 	kmeans_iterations: usize,
	surrogate_epochs: usize, 	surrogate_rate: f64, 	initial: f64, 	beta1: f64, 	beta2: f64, 	epsilon: f64, 	decay: f64,
	rat_batch: usize, 	tile: Tile, 	tile_limit: Tile, 	activation: [f64; 8], } impl Config {
	fn load() -> Result<Self> { 		let tile = Tile::load(false)?;
		let tile_limit = Tile::load(true)?;
		require(tile.m <= tile_limit.m && tile.n <= tile_limit.n && tile.k <= tile_limit.k,
			"configured contraction tile exceeds its maximum")?; 		Ok(Self {
			kmeans_iterations: natural("kmeans iterations", env!("RECIPE_KMEANS_ITERATIONS"))?,
			surrogate_epochs: natural("surrogate epochs", env!("RECIPE_SURROGATE_EPOCHS"))?,
			surrogate_rate: number("surrogate rate", env!("RECIPE_SURROGATE_RATE"))?,
			rat_batch: natural("RAT batch rows", env!("RECIPE_RAT_BATCH_ROWS"))?, 			tile, 			tile_limit,
			initial: number("initial weight", env!("RECIPE_TRAIN_INITIAL_WEIGHT"))?,
			beta1: number("AdamW beta1", env!("RECIPE_ADAMW_BETA1"))?,
			beta2: number("AdamW beta2", env!("RECIPE_ADAMW_BETA2"))?,
			epsilon: number("AdamW epsilon", env!("RECIPE_ADAMW_EPSILON"))?,
			decay: number("AdamW weight decay", env!("RECIPE_ADAMW_WEIGHT_DECAY"))?, 			activation: [
				number("leak slope", env!("RECIPE_LEAK_SLOPE"))?, 				number("PReLU slope", env!("RECIPE_PRELU_SLOPE"))?,
				number("ELU alpha", env!("RECIPE_ELU_ALPHA"))?, 				number("SELU alpha", env!("RECIPE_SELU_ALPHA"))?,
				number("SELU scale", env!("RECIPE_SELU_SCALE"))?, 				number("GELU scale", env!("RECIPE_GELU_SCALE"))?,
				number("GELU cubic", env!("RECIPE_GELU_CUBIC"))?, 				number("Huber threshold", env!("RECIPE_HUBER_THRESHOLD"))?,
			], 		}) 	} } fn number(name: &str, text: &str) -> Result<f64> {
	let value = text.parse::<f64>().map_err(|error| RecipeError::new(format!("invalid {name}: {error}")))?;
	(value.is_finite() && value > 0.0) 		.then_some(value)
		.ok_or_else(|| RecipeError::new(format!("{name} must be finite and positive"))) }
fn natural(name: &str, text: &str) -> Result<usize> {
	let value = text.parse::<usize>().map_err(|error| RecipeError::new(format!("invalid {name}: {error}")))?;
	require(value != 0, format!("{name} must be positive")).map(|_| value) }
fn stored_graph(graph: &Graph, data: &Data) -> bundle::StoredGraph {
	let inputs = (0..graph.input.elements()).map(|index| format!("input{index}")).collect();
	let output = data.target.first().cloned().unwrap_or_else(|| "target".to_owned());
	bundle::StoredGraph { graph: graph.clone(), inputs, outputs: vec![output] } }
fn save_graph(path: &str, graph: &Graph, data: &Data, schema: &str, weights: &[f64]) -> Result<()> {
	let mut stored = stored_graph(graph, data);
	stored.graph.parameters = weights.to_vec();
	bundle::save(path, schema, &[stored]) } struct DeviceTape { 	gpu: &'static Gpu,
	values: Vec<Buffer>, 	_contexts: Vec<Buffer>, 	_adjoints: Vec<Buffer>, 	samples: Buffer,
	input_adjoint: Buffer, 	targets: Buffer,
	weights: Buffer, 	frozen: Buffer, 	best: Buffer, 	moments: Buffer, 	variances: Buffer, 	gradient: Buffer,
	metrics: Buffer, 	best_loss: Buffer, 	value_pointers: Buffer, 	context_pointers: Buffer, 	adjoint_pointers: Buffer,
	descriptors: Buffer, 	arguments: Buffer, 	rows: u32, 	nodes: u32, 	parameters: u32, 	threads: u32, 	input: usize,
	output: usize, 	capacity: usize, 	tile: Tile, }
impl DeviceTape { 	fn new(
		graph: &Graph, samples: &[f64], targets: &[f64], backend: Backend, tile: Tile,
	) -> Result<Self> {
		let inputs = graph.input.elements();
		require(inputs != 0 && !samples.is_empty() && samples.len() % inputs == 0, "model input batch is invalid")?;
		let rows = samples.len() / inputs;
		require(
			targets.is_empty() || targets.len() == rows || targets.len() == rows * graph.output.elements(),
			"target batch is invalid",
		)?;
		let gpu = gpu(backend)?; 		let (mut descriptors, mut arguments, mut values, mut contexts, mut adjoints) =
			(Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
		let program_base = checked_mul(graph.nodes.len(), 9, "node arguments")?; 		for node in &graph.nodes {
			descriptors.extend(node_descriptor(node, program_base)?);
			arguments.extend(node.argument);
			let elements = graph_rows_buffer(node.output, rows)?;
			values.push(Buffer::new(gpu, elements)?);
			let empty = std::iter::repeat_n(0_u8, elements).collect::<Vec<_>>();
			adjoints.push(Buffer::upload(gpu, &empty)?);
			contexts.push(Buffer::new(gpu, node_context(node, rows)?)?); 		}
		arguments.extend(&graph.programs);
		let addresses = |buffers: &[Buffer]| buffers.iter().map(|buffer| buffer.pointer).collect::<Vec<_>>();
		let zeros = std::iter::repeat_n(0.0, graph.parameters.len().max(1)).collect::<Vec<_>>();
		let target_values = if targets.is_empty() { vec![0.0] } else { targets.to_vec() }; 		Ok(Self { 			gpu,
			value_pointers: Buffer::upload(gpu, &addresses(&values))?,
			context_pointers: Buffer::upload(gpu, &addresses(&contexts))?,
			adjoint_pointers: Buffer::upload(gpu, &addresses(&adjoints))?, 			descriptors: Buffer::upload(gpu, &descriptors)?,
			arguments: Buffer::upload(gpu, &arguments)?, 			samples: Buffer::upload(gpu, samples)?,
			input_adjoint: Buffer::upload(gpu, &std::iter::repeat_n(0.0, samples.len()).collect::<Vec<_>>())?,
			targets: Buffer::upload(gpu, &target_values)?, 			weights: Buffer::upload(gpu, &graph.parameters)?,
			frozen: Buffer::upload(gpu, if graph.frozen.is_empty() { &[1] } else { &graph.frozen })?,
			best: Buffer::upload(gpu, &graph.parameters)?, 			moments: Buffer::upload(gpu, &zeros)?,
			variances: Buffer::upload(gpu, &zeros)?, 			gradient: Buffer::upload(gpu, &zeros)?,
			metrics: Buffer::upload(gpu, &[0.0, 0.0, 0.0])?,
			best_loss: Buffer::upload(gpu, &[f64::INFINITY, f64::NAN, f64::NAN, f64::INFINITY])?,
			rows: narrow(rows, "GPU rows")? as u32, 			nodes: narrow(graph.nodes.len(), "GPU nodes")? as u32,
			parameters: narrow(graph.parameters.len(), "GPU parameters")? as u32, 			threads: 0,
			input: graph.input.elements(), 			output: graph.output.elements(), 			values, 			_contexts: contexts,
			_adjoints: adjoints, 			capacity: rows, 			tile, 		}) 	}
	fn forward(&mut self) -> Result<()> { 		let dispatch = self.gpu.forward;
		self.threads = dispatch.geometry.threads(self.rows)?;
		let mut arguments = self.forward_arguments();
		self.gpu.launch(dispatch, &mut arguments, self.threads) 	}
	fn forward_arguments(&mut self) -> [*mut c_void; 12] { 		[
			&mut self.samples.pointer as *mut _ as Ptr, 			&mut self.weights.pointer as *mut _ as Ptr,
			&mut self.value_pointers.pointer as *mut _ as Ptr, 			&mut self.context_pointers.pointer as *mut _ as Ptr,
			&mut self.descriptors.pointer as *mut _ as Ptr, 			&mut self.arguments.pointer as *mut _ as Ptr,
			&mut self.rows as *mut _ as Ptr, 			&mut self.nodes as *mut _ as Ptr, 			&mut self.threads as *mut _ as Ptr,
			&mut self.tile.m as *mut _ as Ptr, 			&mut self.tile.n as *mut _ as Ptr,
			&mut self.tile.k as *mut _ as Ptr, 		] 	}
	fn predictions(&self) -> Result<Vec<f64>> { 		self.values 			.last()
			.ok_or_else(|| RecipeError::new("GPU tape is empty"))? 			.download(self.rows as usize * self.output) 	} 	fn epoch(
		&mut self, 		step: usize, 		rate: f64, 		loss: LossFunction, 		tolerance: f64, 		config: Config,
		direct: bool, 	) -> Result<(f64, bool)> { 		let mut loss = if direct { 7 } else { loss.0 as u32 };
		let mut huber_threshold = config.activation[7];
		let mut step = narrow(step, "optimizer step")? as u32;
		let mut rate = rate;
		let mut beta1 = config.beta1;
		let mut beta2 = config.beta2;
		let mut beta1_power = beta1.powi(step as i32);
		let mut beta2_power = beta2.powi(step as i32);
		let mut epsilon = config.epsilon;
		let mut decay = config.decay;
		let mut tolerance = tolerance; 		let mut call = [ 			&mut self.samples.pointer as *mut _ as Ptr,
			&mut self.input_adjoint.pointer as *mut _ as Ptr,
			&mut self.targets.pointer as *mut _ as Ptr, 			&mut self.weights.pointer as *mut _ as Ptr,
			&mut self.frozen.pointer as *mut _ as Ptr, 			&mut self.best.pointer as *mut _ as Ptr,
			&mut self.value_pointers.pointer as *mut _ as Ptr, 			&mut self.context_pointers.pointer as *mut _ as Ptr,
			&mut self.adjoint_pointers.pointer as *mut _ as Ptr, 			&mut self.descriptors.pointer as *mut _ as Ptr,
			&mut self.arguments.pointer as *mut _ as Ptr, 			&mut self.metrics.pointer as *mut _ as Ptr,
			&mut self.gradient.pointer as *mut _ as Ptr, 			&mut self.moments.pointer as *mut _ as Ptr,
			&mut self.variances.pointer as *mut _ as Ptr, 			&mut self.best_loss.pointer as *mut _ as Ptr,
			&mut self.rows as *mut _ as Ptr, 			&mut self.nodes as *mut _ as Ptr, 			&mut self.parameters as *mut _ as Ptr,
			&mut loss as *mut _ as Ptr, 			&mut huber_threshold as *mut _ as Ptr, 			&mut rate as *mut _ as Ptr,
			&mut beta1 as *mut _ as Ptr, 			&mut beta2 as *mut _ as Ptr, 			&mut beta1_power as *mut _ as Ptr,
			&mut beta2_power as *mut _ as Ptr, 			&mut epsilon as *mut _ as Ptr, 			&mut decay as *mut _ as Ptr,
			&mut tolerance as *mut _ as Ptr, 			&mut step as *mut _ as Ptr, 			&mut self.threads as *mut _ as Ptr,
			&mut self.tile.m as *mut _ as Ptr, 			&mut self.tile.n as *mut _ as Ptr,
			&mut self.tile.k as *mut _ as Ptr, 		];
		let dispatch = self.gpu.epoch;
		self.threads = dispatch.geometry.threads(self.rows)?;
		self.gpu.launch(dispatch, &mut call, self.threads)?;
		let metrics = self.metrics.download::<f64>(3)?; 		Ok((metrics[0], metrics[1] != 0.0)) 	}
	fn activate(&mut self, rows: usize) -> Result<()> {
		require(rows != 0 && rows <= self.capacity, "active GPU batch exceeds its allocation")?;
		self.rows = narrow(rows, "active GPU rows")? as u32; 		Ok(()) 	}
	fn write_samples(&self, row: usize, values: &[f64]) -> Result<()> {
		require(values.len() == self.input && row < self.capacity, "RAT input row has the wrong width")?;
		self.samples.write(row * self.input, values) 	}
	fn write_targets(&self, row: usize, values: &[f64]) -> Result<()> {
		require(values.len() == self.output && row < self.capacity, "RAT target row has the wrong width")?;
		self.targets.write(row * self.output, values) 	}
	fn node_row(&self, node: usize, row: usize, width: usize) -> Result<Vec<f64>> {
		require(node < self.values.len() && row < self.rows as usize, "RAT proposal row is absent")?;
		self.values[node].download_range(row * width, width) 	}
	fn weights(&self, best: bool) -> Result<Vec<f64>> { 		if best { 			self.best.download(self.parameters as usize)
		} else { 			self.weights.download(self.parameters as usize) 		} 	} 	fn restore_best(&mut self) -> Result<()> {
		self.weights = Buffer::upload(self.gpu, &self.weights(true)?)?; 		Ok(()) 	} }
fn node_descriptor(node: &Node, program_base: usize) -> Result<[i32; 11]> {
	let program_offset = if node.program_count == 0 { 		0 	} else {
		checked_add(program_base, node.program_offset, "scalar program offset")? 	}; 	Ok([ 		node.op as i32, 		node.source,
		node.second, 		narrow(node.input.channels, "input channels")?, 		narrow(node.input.length, "input length")?,
		narrow(node.output.channels, "output channels")?, 		narrow(node.output.length, "output length")?,
		narrow(node.offset, "weight offset")?, 		narrow(node.parameters, "parameter count")?,
		narrow(program_offset, "program offset")?, 		narrow(node.program_count, "scalar instruction count")?, 	]) }
fn graph_rows_buffer(shape: Shape, rows: usize) -> Result<usize> {
	checked_mul(checked_mul(rows, shape.elements(), "node elements")?, size_of::<f64>(), "node bytes") }
fn node_context(node: &Node, rows: usize) -> Result<usize> { 	let elements = match node.op {
		Primitive::Elementwise => checked_mul( 			2 * node.program_count,
			checked_mul(rows, node.output.elements(), "program batch")?, 			"program", 		)?, 		Primitive::Attention => {
			checked_mul(6, checked_mul(rows, node.output.elements(), "attention context")?, "attention")? 		}
		Primitive::Scan => { 			let state_count = checked_mul(rows, node.output.elements(), "scan batch")?;
			let gates = node.argument[0] as usize;
			let states = checked_mul(2 * gates + 1, state_count, "scan states")?;
			let gradients = checked_mul(rows, node.parameters, "scan gradients")?;
			checked_add(states, checked_add(gradients, 2 * rows * node.output.channels, "scan scratch")?, "scan")? 		}
		Primitive::Pool => checked_mul(rows, node.output.elements(), "pool context")?, 		Primitive::Normalize => {
			let groups = node.output.channels.max(checked_mul(rows, node.output.length, "layer groups")?);
			checked_mul(4, groups, "normalization context")? 		} 		_ => 1, 	};
	checked_mul(elements.max(1), size_of::<f64>(), "context bytes") } fn narrow(value: usize, role: &str) -> Result<i32> {
	i32::try_from(value).map_err(|_| RecipeError::new(format!("{role} exceeds i32"))) }
struct Buffer {
	runtime: &'static Gpu, 	pointer: u64, 	bytes: usize, }
impl Buffer { 	fn new(runtime: &'static Gpu, bytes: usize) -> Result<Self> {
		Ok(Self { runtime, pointer: runtime.allocate(bytes)?, bytes }) 	}
	fn upload<T>(runtime: &'static Gpu, values: &[T]) -> Result<Self> {
		let buffer = Self::new(runtime, size_of_val(values))?;
		runtime.upload(buffer.pointer, values.as_ptr().cast(), size_of_val(values))?; 		Ok(buffer) 	}
	fn write<T>(&self, offset: usize, values: &[T]) -> Result<()> {
		let start = checked_mul(offset, size_of::<T>(), "GPU write offset")?;
		require(checked_add(start, size_of_val(values), "GPU write")? <= self.bytes, "GPU write exceeds buffer")?;
		self.runtime.upload(self.pointer + start as u64, values.as_ptr().cast(), size_of_val(values)) 	}
	fn download<T: Copy + Default>(&self, count: usize) -> Result<Vec<T>> { 		self.download_range(0, count) 	}
	fn download_range<T: Copy + Default>(&self, offset: usize, count: usize) -> Result<Vec<T>> {
		let start = checked_mul(offset, size_of::<T>(), "GPU read offset")?;
		let mut values = std::iter::repeat_n(T::default(), count).collect::<Vec<_>>();
		require(checked_add(start, size_of_val(&*values), "GPU read")? <= self.bytes, "GPU read exceeds buffer")?;
		self.runtime.synchronize()?;
		self.runtime.download(values.as_mut_ptr().cast(), self.pointer + start as u64, size_of_val(&*values))?;
		Ok(values) 	} }
impl Drop for Buffer { 	fn drop(&mut self) { 		self.runtime.free(self.pointer); 	} } #[derive(Clone, Copy)]
struct Kernel { 	object: u64, 	#[cfg(feature = "amd")] 	kernarg: usize, 	#[cfg(feature = "amd")] 	group: u32,
	#[cfg(feature = "amd")] 	private: u32, 	#[cfg(feature = "amd")] 	layout: &'static [u8], }
#[derive(Clone, Copy)] struct Dispatch { 	kernel: Kernel, 	geometry: Geometry, }
const FORWARD_ARGS: &[u8] = b"888888444444";
const EPOCH_ARGS: &[u8] = b"8888888888888888444488888888844444"; #[cfg(feature = "nvidia")] struct Cuda {
	allocate: unsafe extern "C" fn(*mut u64, usize) -> i32, 	free: unsafe extern "C" fn(u64) -> i32,
	upload: unsafe extern "C" fn(u64, *const c_void, usize) -> i32,
	download: unsafe extern "C" fn(Ptr, u64, usize) -> i32, 	synchronize: unsafe extern "C" fn() -> i32,
	launch: unsafe extern "C" fn(usize, u32, u32, u32, u32, u32, u32, u32, Ptr, *mut Ptr) -> i32, }
#[cfg(feature = "nvidia")] impl Kernel { 	const fn cuda(object: usize, _layout: &'static [u8]) -> Self { 		Self {
			object: object as u64, 			#[cfg(feature = "amd")] 			kernarg: 0, 			#[cfg(feature = "amd")] 			group: 0,
			#[cfg(feature = "amd")] 			private: 0, 			#[cfg(feature = "amd")] 			layout: _layout, 		} 	} }
#[cfg(feature = "amd")] #[allow(dead_code)] struct Hsa {
	allocate: unsafe extern "C" fn(u64, usize, u32, *mut Ptr) -> i32, 	free: unsafe extern "C" fn(Ptr) -> i32,
	allow: unsafe extern "C" fn(u32, *const u64, *const u32, *const c_void) -> i32,
	copy: unsafe extern "C" fn(Ptr, *const c_void, usize) -> i32, 	store: unsafe extern "C" fn(u64, i64),
	wait: unsafe extern "C" fn(u64, i32, i64, u64, i32) -> i64, 	write: unsafe extern "C" fn(*const HsaQueue, u64) -> u64,
	queue: Ptr, 	signal: u64, 	cpu_agent: u64, 	vram_pool: u64, 	kernarg_pool: u64, 	kernarg_size: usize, 	kernarg: Ptr,
	_code: fs::File, } enum Driver { 	#[cfg(feature = "amd")] 	Hsa(Hsa), 	#[cfg(feature = "nvidia")] 	Cuda(Cuda), }
#[allow(dead_code)] struct Gpu {
	backend: Backend, 	driver: Driver, 	forward: Dispatch, 	epoch: Dispatch, 	dispatch: Mutex<()>, }
unsafe impl Send for Gpu {}
unsafe impl Sync for Gpu {}
#[cfg(feature = "amd")]
#[repr(C)] struct HsaQueue { 	kind: u32, 	features: u32, 	base: Ptr, 	doorbell: u64, 	size: u32, 	reserved: u32,
	id: u64, } #[cfg(feature = "amd")] #[repr(C)] struct HsaPacket { 	header: u16, 	setup: u16, 	workgroup_x: u16,
	workgroup_y: u16, 	workgroup_z: u16, 	reserved0: u16, 	grid_x: u32, 	grid_y: u32, 	grid_z: u32, 	private: u32,
	group: u32, 	object: u64, 	kernarg: Ptr, 	reserved1: u64, 	completion: u64, } #[cfg(feature = "nvidia")]
type Count = unsafe extern "C" fn(*mut i32) -> i32; #[cfg(feature = "nvidia")]
type Attribute = unsafe extern "C" fn(*mut i32, i32, i32) -> i32; #[cfg(feature = "nvidia")]
type Device = unsafe extern "C" fn(*mut i32, i32) -> i32; #[cfg(feature = "nvidia")]
type Context = unsafe extern "C" fn(*mut Ptr, u32, i32) -> i32; #[cfg(feature = "nvidia")]
type Module = unsafe extern "C" fn(*mut Ptr, *const u8) -> i32; #[cfg(feature = "nvidia")]
type Function = unsafe extern "C" fn(*mut usize, Ptr, *const u8) -> i32;
#[cfg(feature = "nvidia")]
type FunctionAttribute = unsafe extern "C" fn(*mut i32, i32, usize) -> i32;
#[cfg(feature = "nvidia")]
type Occupancy = unsafe extern "C" fn(*mut i32, usize, i32, usize) -> i32;
#[cfg(any(feature = "amd", feature = "nvidia"))] struct Library(Ptr); #[cfg(any(feature = "amd", feature = "nvidia"))]
impl Library { 	fn open(name: &str) -> Result<Self> { 		let name = format!("{name}\0");
		let handle = unsafe { dlopen(name.as_ptr().cast(), 2) };
		require(!handle.is_null(), format!("cannot load {name:?}"))?; 		Ok(Self(handle)) 	}
	fn function<F: Copy>(&self, name: &[u8]) -> Result<F> { 		let pointer = unsafe { dlsym(self.0, name.as_ptr().cast()) };
		require(!pointer.is_null(), format!("runtime symbol {:?} is absent", name))?;
		Ok(unsafe { std::mem::transmute_copy(&pointer) }) 	} }
fn driver_status(backend: Backend, status: i32, action: &str) -> Result<()> {
	(status == 0).then_some(()).ok_or_else(|| RecipeError::new(format!("{backend:?} {action} failed: {status}"))) }
impl Gpu { 	fn status(&self, status: i32, action: &str) -> Result<()> { 		driver_status(self.backend, status, action) 	}
	fn allocate(&self, bytes: usize) -> Result<u64> {
		unsafe { 			match &self.driver { 				#[cfg(feature = "nvidia")] 				Driver::Cuda(driver) => {
					let mut pointer = 0;
					self.status((driver.allocate)(&mut pointer, bytes), "allocation")?; 					Ok(pointer) 				}
				#[cfg(feature = "amd")] 				Driver::Hsa(driver) => { 					let mut pointer = ptr::null_mut();
					self.status((driver.allocate)(driver.vram_pool, bytes, 0, &mut pointer), "allocation")?; 					self.status(
						(driver.allow)(1, &driver.cpu_agent, ptr::null(), pointer), 						"CPU allocation access", 					)?;
					Ok(pointer as u64) 				} 			} 		} 	} 	fn free(&self, pointer: u64) { 		unsafe { 			match &self.driver {
				#[cfg(feature = "nvidia")] 				Driver::Cuda(driver) => { 					(driver.free)(pointer); 				}
				#[cfg(feature = "amd")] 				Driver::Hsa(driver) => { 					(driver.free)(pointer as Ptr); 				} 			} 		} 	}
	fn upload(&self, dst: u64, src: *const c_void, bytes: usize) -> Result<()> { 		unsafe { 			match &self.driver {
				#[cfg(feature = "nvidia")] 				Driver::Cuda(driver) => self.status((driver.upload)(dst, src, bytes), "upload"),
				#[cfg(feature = "amd")] 				Driver::Hsa(driver) => self.status((driver.copy)(dst as Ptr, src, bytes), "upload"),
			} 		} 	} 	fn download(&self, dst: Ptr, src: u64, bytes: usize) -> Result<()> { 		unsafe { 			match &self.driver {
				#[cfg(feature = "nvidia")] 				Driver::Cuda(cuda) => self.status((cuda.download)(dst, src, bytes), "download"),
				#[cfg(feature = "amd")]
				Driver::Hsa(driver) => self.status((driver.copy)(dst, src as *const c_void, bytes), "download"), 			} 		} 	}
	fn synchronize(&self) -> Result<()> { 		unsafe { 			match &self.driver { 				#[cfg(feature = "nvidia")]
				Driver::Cuda(driver) => self.status((driver.synchronize)(), "synchronization"), 				#[cfg(feature = "amd")]
				Driver::Hsa(driver) => require((driver.wait)(driver.signal, 0, 0, u64::MAX, 1) == 0, "AMD synchronization failed"),
			} 		} 	} 	fn launch(
		&self, dispatch: Dispatch, arguments: &mut [Ptr], threads: u32,
	) -> Result<()> {
		require(!INTERRUPTED.load(Ordering::Acquire), "interrupted before GPU dispatch")?;
		let block = dispatch.geometry.block;
		let kernel = dispatch.kernel;
		let _guard = self.dispatch.lock().map_err(|_| RecipeError::new("GPU dispatch lock is poisoned"))?; 		unsafe {
			match &self.driver { 				#[cfg(feature = "nvidia")] 				Driver::Cuda(driver) => { 					let stream = ptr::null_mut();
					self.status( 						(driver.launch)( 							kernel.object as usize,
							threads / block, 							1, 							1, 							block,
							1, 							1, 							0, 							stream, 							arguments.as_mut_ptr(), 						),
						"dispatch", 					) 				} 				#[cfg(feature = "amd")] 				Driver::Hsa(driver) => {
					require(arguments.len() == kernel.layout.len(), "kernel argument count is invalid")?;
					ptr::write_bytes(driver.kernarg.cast::<u8>(), 0, driver.kernarg_size);
					let mut offset = 0; 					for (argument, kind) in arguments.iter().zip(kernel.layout) {
						let bytes = usize::from(*kind - b'0'); 						ptr::copy_nonoverlapping( 							(*argument).cast::<u8>(),
							driver.kernarg.cast::<u8>().add(offset), 							bytes, 						);
						offset += bytes; 					} 					require( 						offset <= kernel.kernarg && kernel.kernarg <= driver.kernarg_size,
						"kernarg layout is invalid", 					)?;
					(driver.store)(driver.signal, 1);
					let queue = &mut *(driver.queue as *mut HsaQueue);
					let index = (driver.write)(queue, 1); 					let packet =
						queue.base.cast::<HsaPacket>().add(index as usize & (queue.size as usize - 1)); 					packet.write(HsaPacket {
						header: 0, 						setup: 1, 						workgroup_x: block as u16,
						workgroup_y: 1, 						workgroup_z: 1,
						reserved0: 0, 						grid_x: threads, 						grid_y: 1, 						grid_z: 1, 						private: kernel.private,
						group: kernel.group, 						object: kernel.object, 						kernarg: driver.kernarg, 						reserved1: 0,
						completion: driver.signal, 					});
					std::sync::atomic::fence(Ordering::Release);
					let header = &*(&mut (*packet).header as *mut u16 as *mut std::sync::atomic::AtomicU16);
					header.store(2 | 2 << 9 | 2 << 11, Ordering::Release);
					(driver.store)(queue.doorbell, index as i64);
					require((driver.wait)(driver.signal, 0, 0, u64::MAX, 1) == 0, "AMD dispatch failed") 				} 			} 		} 	} }
static AMD: OnceLock<Result<Gpu>> = OnceLock::new();
static NVIDIA: OnceLock<Result<Gpu>> = OnceLock::new(); fn device_backend() -> Result<Backend> {
	let mut failures = Vec::new(); 	for backend in [Backend::Amd, Backend::Nvidia] { 		match gpu(backend) {
			Ok(_) => return Ok(backend), 			Err(error) => failures.push(error.to_string()), 		} 	}
	Err(RecipeError::new(failures.join("; "))) } fn gpu(backend: Backend) -> Result<&'static Gpu> {
	let loaded = match backend { 		Backend::Amd => AMD.get_or_init(load_amd),
		Backend::Nvidia => NVIDIA.get_or_init(load_nvidia), 	}; 	loaded.as_ref().map_err(Clone::clone) }
#[cfg(feature = "nvidia")] fn discrete(count: i32, mut probe: impl FnMut(i32) -> Result<Option<i32>>) -> Result<i32> {
	(0..count) 		.map(&mut probe) 		.find_map(|result| result.transpose()) 		.transpose()?
		.ok_or_else(|| RecipeError::new("Nvidia has no discrete GPU")) } #[cfg(feature = "amd")]
type HsaInfo = unsafe extern "C" fn(u64, i32, Ptr) -> i32; #[cfg(feature = "amd")] struct HsaQuery { 	info: HsaInfo,
	attribute: i32, 	expected: u32, 	secondary: i32, 	mask: u32, 	found: u64, } #[cfg(feature = "amd")]
extern "C" fn collect_hsa(handle: u64, pointer: Ptr) -> i32 { 	unsafe { 		let query = &mut *pointer.cast::<HsaQuery>();
		let mut value = 0;
		let mut status = (query.info)(handle, query.attribute, (&mut value as *mut u32).cast());
		if status != 0 || value != query.expected { 			return status; 		} 		if query.secondary >= 0 {
			status = (query.info)(handle, query.secondary, (&mut value as *mut u32).cast());
			if status != 0 || value & query.mask == 0 { 				return status; 			} 		} 		if query.found == 0 {
			query.found = handle; 		} 		0 	} } #[cfg(feature = "amd")] struct HsaGpuQuery { 	info: HsaInfo, 	found: u64, }
#[cfg(feature = "amd")] extern "C" fn collect_discrete_hsa(handle: u64, pointer: Ptr) -> i32 { 	unsafe {
		let query = &mut *pointer.cast::<HsaGpuQuery>();
		let mut device = 0_u32;
		let mut status = (query.info)(handle, 17, (&mut device as *mut u32).cast()); 		if status != 0 || device != 1 {
			return status; 		}
		let mut properties = 0_u64;
		status = (query.info)(handle, 0xA114, (&mut properties as *mut u64).cast()); 		if status != 0 || properties & 1 != 0 {
			return status; 		} 		if query.found == 0 {
			query.found = handle; 		} 		0 	} } #[cfg(feature = "amd")]
type HsaSymbol = unsafe extern "C" fn(u64, *const u8, *const u64, *mut u64) -> i32; #[cfg(feature = "amd")]
type HsaSymbolInfo = unsafe extern "C" fn(u64, i32, Ptr) -> i32; #[cfg(feature = "amd")] unsafe fn hsa_kernel(
	symbol: HsaSymbol, 	info: HsaSymbolInfo, 	executable: u64, 	agent: u64, 	name: &'static [u8], 	layout: &'static [u8],
) -> Result<Kernel> { 	let mut handle = 0;
	driver_status(Backend::Amd, unsafe { symbol(executable, name.as_ptr(), &agent, &mut handle) }, "kernel lookup")?;
	let mut kernel = Kernel { object: 0, kernarg: 0, group: 0, private: 0, layout }; 	for (attribute, output) in [
		(22, (&mut kernel.object as *mut u64).cast()), 		(11, (&mut kernel.kernarg as *mut usize).cast()),
		(13, (&mut kernel.group as *mut u32).cast()), 		(14, (&mut kernel.private as *mut u32).cast()), 	] {
		driver_status(Backend::Amd, unsafe { info(handle, attribute, output) }, "kernel metadata")?; 	} 	Ok(kernel) }
#[cfg(feature = "amd")] fn kfd_property(text: &str, name: &str) -> Result<u32> { 	text.lines()
	.find_map(|line| line.split_once(' ').filter(|value| value.0 == name)).ok_or_else(||
		RecipeError::new(format!("KFD property {name:?} is absent")))?.1.parse::<u32>()
	.map_err(|error| RecipeError::new(format!("KFD property {name:?} is invalid: {error}"))) }
fn load_amd() -> Result<Gpu> { 	#[cfg(not(feature = "amd"))]
	return Err(RecipeError::new("AMD support is not compiled into this build")); 	#[cfg(feature = "amd")] 	unsafe {
		let runtime = Library::open(env!("RECIPE_HSA_RUNTIME"))?;
		let init: unsafe extern "C" fn() -> i32 = runtime.function(b"hsa_init\0")?;
		let iterate: unsafe extern "C" fn(extern "C" fn(u64, Ptr) -> i32, Ptr) -> i32 =
			runtime.function(b"hsa_iterate_agents\0")?;
		let info: HsaInfo = runtime.function(b"hsa_agent_get_info\0")?;
		driver_status(Backend::Amd, init(), "initialization")?;
		let mut cpu = HsaQuery { info, attribute: 17, expected: 0, secondary: -1, mask: 0, found: 0 };
		let mut gpu = HsaGpuQuery { info, found: 0 };
		driver_status(Backend::Amd, iterate(collect_hsa, (&mut cpu as *mut HsaQuery).cast()), "CPU agent")?; 		driver_status(
			Backend::Amd, 			iterate(collect_discrete_hsa, (&mut gpu as *mut HsaGpuQuery).cast()), 			"GPU agent", 		)?;
		require(cpu.found != 0 && gpu.found != 0, "AMD CPU or discrete GPU agent is absent")?;
		let pool_info: HsaInfo = runtime.function(b"hsa_amd_memory_pool_get_info\0")?;
		let pool_iterate: unsafe extern "C" fn(u64, extern "C" fn(u64, Ptr) -> i32, Ptr) -> i32 =
			runtime.function(b"hsa_amd_agent_iterate_memory_pools\0")?;
		let mut vram = HsaQuery { info: pool_info, attribute: 0, expected: 0, secondary: 1, mask: 4, found: 0 };
		let mut kernarg = HsaQuery { info: pool_info, attribute: 0, expected: 0, secondary: 1, mask: 1, found: 0 };
		driver_status( 			Backend::Amd, 			pool_iterate(gpu.found, collect_hsa, (&mut vram as *mut HsaQuery).cast()),
			"VRAM pools", 		)?; 		driver_status( 			Backend::Amd,
			pool_iterate(cpu.found, collect_hsa, (&mut kernarg as *mut HsaQuery).cast()), 			"KERNARG pools", 		)?;
		require(vram.found != 0 && kernarg.found != 0, "AMD VRAM or KERNARG pool is absent")?;
		let (mut wave, mut workgroup, mut available, mut node, mut waves, mut simds, mut cus) =
			(0_u32, 0_u32, 0_u32, 0_u32, 0_u32, 0_u32, 0_u32);
		for (attribute, output, action) in [
			(6, (&mut wave as *mut u32).cast(), "wave query"),
			(8, (&mut workgroup as *mut u32).cast(), "workgroup query"),
			(0xA002, (&mut available as *mut u32).cast(), "CU query"),
			(0xA004, (&mut node as *mut u32).cast(), "KFD node query"),
			(0xA00A, (&mut waves as *mut u32).cast(), "wave occupancy query"),
			(0xA00B, (&mut simds as *mut u32).cast(), "SIMD query"),
			(0xA014, (&mut cus as *mut u32).cast(), "cooperative CU query"),
		] {
			driver_status(Backend::Amd, info(gpu.found, attribute, output), action)?;
		}
		require(cus <= available, "AMD cooperative CU count exceeds available CUs")?;
		let code = fs::File::open(env!("RECIPE_HSA_CODE_OBJECT"))
			.map_err(|error| RecipeError::new(format!("cannot open HSA code object: {error}")))?;
		let reader_create: unsafe extern "C" fn(i32, *mut u64) -> i32 =
			runtime.function(b"hsa_code_object_reader_create_from_file\0")?;
		let executable_create: unsafe extern "C" fn(i32, i32, Ptr, *mut u64) -> i32 =
			runtime.function(b"hsa_executable_create_alt\0")?;
		let executable_load: unsafe extern "C" fn(u64, u64, u64, Ptr, Ptr) -> i32 =
			runtime.function(b"hsa_executable_load_agent_code_object\0")?;
		let executable_freeze: unsafe extern "C" fn(u64, Ptr) -> i32 = 			runtime.function(b"hsa_executable_freeze\0")?;
		let symbol: HsaSymbol = runtime.function(b"hsa_executable_get_symbol_by_name\0")?;
		let symbol_info: HsaSymbolInfo = runtime.function(b"hsa_executable_symbol_get_info\0")?;
		let (mut reader, mut executable) = (0, 0);
		let descriptor = std::os::fd::AsRawFd::as_raw_fd(&code);
		driver_status(Backend::Amd, reader_create(descriptor, &mut reader), "code-object reader")?; 		driver_status(
			Backend::Amd, 			executable_create(1, 0, ptr::null_mut(), &mut executable), 			"executable creation", 		)?;
		driver_status( 			Backend::Amd, 			executable_load(executable, gpu.found, reader, ptr::null_mut(), ptr::null_mut()),
			"code-object load", 		)?;
		driver_status(Backend::Amd, executable_freeze(executable, ptr::null_mut()), "executable freeze")?;
		let forward = hsa_kernel(symbol, symbol_info, executable, gpu.found, b"forward_graph.kd\0", FORWARD_ARGS)?;
		let epoch = hsa_kernel(symbol, symbol_info, executable, gpu.found, b"tape_epoch_graph.kd\0", EPOCH_ARGS)?;
		let compiled = |name, text| -> Result<u32> { Ok(narrow(natural(name, text)?, name)? as u32) };
		let forward_resources = Resources { 			registers: compiled(
				"HSA forward VGPRs", env!("RECIPE_HSA_FORWARD_VGPRS"))?, 			shared: forward.group,
			max_block: compiled("HSA forward workgroup", env!("RECIPE_HSA_FORWARD_MAX_BLOCK"))?, 		};
		let epoch_resources = Resources { 			registers: compiled(
				"HSA epoch VGPRs", env!("RECIPE_HSA_EPOCH_VGPRS"))?, 			shared: epoch.group,
			max_block: compiled("HSA epoch workgroup", env!("RECIPE_HSA_EPOCH_MAX_BLOCK"))?, 		};
		let path = format!("/sys/class/kfd/kfd/topology/nodes/{node}/properties");
		let properties = fs::read_to_string(&path)
			.map_err(|error| RecipeError::new(format!("cannot read {path}: {error}")))?;
		let gfx = kfd_property(&properties, "gfx_target_version")?;
		let target = format!("gfx{}{}{}", gfx / 10000, gfx / 100 % 100, gfx % 100);
		require(target == env!("RECIPE_HSA_ARCHITECTURE"), format!("AMD target {target} does not match the kernel"))?;
		let lds = kfd_property(&properties, "lds_size_in_kb")?.checked_mul(1024)
			.ok_or_else(|| RecipeError::new("AMD LDS size overflows"))?;
		let forward_geometry = amd(cus, wave, workgroup, lds, waves, simds, forward_resources)?;
		let epoch_geometry = amd(cus, wave, workgroup, lds, waves, simds, epoch_resources)?;
		let queue_create: unsafe extern "C" fn(u64, u32, u32, Ptr, Ptr, u32, u32, *mut Ptr) -> i32 =
			runtime.function(b"hsa_queue_create\0")?;
		let signal_create: unsafe extern "C" fn(i64, u32, *const u64, *mut u64) -> i32 =
			runtime.function(b"hsa_signal_create\0")?; 		let allocate: unsafe extern "C" fn(u64, usize, u32, *mut Ptr) -> i32 =
			runtime.function(b"hsa_amd_memory_pool_allocate\0")?;
		let allow: unsafe extern "C" fn(u32, *const u64, *const u32, *const c_void) -> i32 =
			runtime.function(b"hsa_amd_agents_allow_access\0")?;
		let (ka_size, mut ka) = (forward.kernarg.max(epoch.kernarg), ptr::null_mut());
		let (mut queue, mut completion) = (ptr::null_mut(), 0); 		driver_status( 			Backend::Amd,
			queue_create(gpu.found, 256, 2, ptr::null_mut(), ptr::null_mut(), u32::MAX, u32::MAX, &mut queue),
			"queue creation", 		)?;
		driver_status(Backend::Amd, signal_create(1, 0, ptr::null(), &mut completion), "signal creation")?;
		driver_status(Backend::Amd, allocate(kernarg.found, ka_size, 0, &mut ka), "KERNARG allocation")?;
		driver_status(Backend::Amd, allow(1, &gpu.found, ptr::null(), ka), "GPU KERNARG access")?;
		eprintln!("AMD forward block {} epoch block {}", forward_geometry.block, epoch_geometry.block);
		Ok(Gpu {
			backend: Backend::Amd, 			driver: Driver::Hsa(Hsa {
				allocate, 				free: runtime.function(b"hsa_amd_memory_pool_free\0")?, 				allow,
				copy: runtime.function(b"hsa_memory_copy\0")?, 				store: runtime.function(b"hsa_signal_store_screlease\0")?,
				wait: runtime.function(b"hsa_signal_wait_scacquire\0")?,
				write: runtime.function(b"hsa_queue_add_write_index_scacq_screl\0")?, 				queue, 				signal: completion,
				cpu_agent: cpu.found, 				vram_pool: vram.found, 				kernarg_pool: kernarg.found, 				kernarg_size: ka_size,
				kernarg: ka, 				_code: code, 			}),
			forward: Dispatch { kernel: forward, geometry: forward_geometry },
			epoch: Dispatch { kernel: epoch, geometry: epoch_geometry },
			dispatch: Mutex::new(()), 		}) 	} }
fn load_nvidia() -> Result<Gpu> { 	#[cfg(not(feature = "nvidia"))]
	return Err(RecipeError::new("NVIDIA support is not compiled into this build")); 	#[cfg(feature = "nvidia")] 	unsafe {
		const MAX_BLOCK: i32 = 1;
		const BLOCK_LDS: i32 = 8;
		const WAVE: i32 = 10;
		const CUS: i32 = 16;
		const INTEGRATED: i32 = 18;
		const THREADS_PER_SM: i32 = 39;
		const SM_LDS: i32 = 81;
		const REGISTERS_PER_SM: i32 = 82;
		const COOPERATIVE: i32 = 95;
		let runtime = Library::open(env!("RECIPE_NV_RUNTIME"))?;
		let init: unsafe extern "C" fn(u32) -> i32 = runtime.function(b"cuInit\0")?;
		let count_devices: Count = runtime.function(b"cuDeviceGetCount\0")?;
		let get_device: Device = runtime.function(b"cuDeviceGet\0")?;
		let attribute: Attribute = runtime.function(b"cuDeviceGetAttribute\0")?;
		let create: Context = runtime.function(b"cuCtxCreate_v2\0")?;
		let load: Module = runtime.function(b"cuModuleLoad\0")?;
		let function: Function = runtime.function(b"cuModuleGetFunction\0")?;
		let function_attribute: FunctionAttribute = runtime.function(b"cuFuncGetAttribute\0")?;
		let occupancy: Occupancy = runtime.function(b"cuOccupancyMaxActiveBlocksPerMultiprocessor\0")?;
		let (mut count, mut forward, mut epoch) = (0, 0, 0);
		let (mut context, mut module) = (ptr::null_mut(), ptr::null_mut());
		driver_status(Backend::Nvidia, init(0), "initialization")?;
		driver_status(Backend::Nvidia, count_devices(&mut count), "device enumeration")?;
		let device = discrete(count, |ordinal| { 			let (mut device, mut integrated) = (0, 0);
			driver_status(Backend::Nvidia, get_device(&mut device, ordinal), "device enumeration")?;
			driver_status(Backend::Nvidia, attribute(&mut integrated, INTEGRATED, device), "device probe")?;
			Ok((integrated == 0).then_some(device)) 		})?;
		let (mut cus, mut wave, mut workgroup, mut block_lds, mut sm_lds, mut registers, mut threads, mut cooperative) =
			(0, 0, 0, 0, 0, 0, 0, 0);
		for (kind, output, action) in [
			(CUS, &mut cus, "SM query"), 			(WAVE, &mut wave, "warp query"),
			(MAX_BLOCK, &mut workgroup, "workgroup query"), 			(BLOCK_LDS, &mut block_lds, "workgroup LDS query"),
			(SM_LDS, &mut sm_lds, "SM LDS query"), 			(REGISTERS_PER_SM, &mut registers, "register query"),
			(THREADS_PER_SM, &mut threads, "resident thread query"),
			(COOPERATIVE, &mut cooperative, "cooperative launch query"),
		] {
			driver_status(Backend::Nvidia, attribute(output, kind, device), action)?;
		}
		require(cooperative != 0, "Nvidia device does not support cooperative launch")?;
		driver_status(Backend::Nvidia, create(&mut context, 0, device), "context creation")?; 		driver_status(
			Backend::Nvidia, 			load(&mut module, concat!(env!("RECIPE_NV_MODULE"), "\0").as_ptr()), 			"module load", 		)?;
		driver_status( 			Backend::Nvidia, 			function(&mut forward, module, b"forward_graph\0".as_ptr()), 			"forward load",
		)?;
		driver_status(Backend::Nvidia, function(&mut epoch, module, b"tape_epoch_graph\0".as_ptr()), "epoch load")?;
		let resource = |kernel| -> Result<Resources> {
			let (mut max_block, mut shared, mut used_registers) = (0, 0, 0);
			for (kind, output, action) in [
				(0, &mut max_block, "kernel workgroup query"), 				(1, &mut shared, "kernel LDS query"),
				(4, &mut used_registers, "kernel register query"),
			] {
				driver_status(Backend::Nvidia, function_attribute(output, kind, kernel), action)?;
			}
			require(max_block > 0 && shared >= 0 && used_registers > 0, "Nvidia kernel resources are invalid")?;
			Ok(Resources { registers: used_registers as u32, shared: shared as u32, max_block: max_block as u32 })
		};
		let forward_resource = resource(forward)?;
		let epoch_resource = resource(epoch)?;
		let geometry = |resources: Resources| -> Result<Geometry> {
			let register_wave = resources.registers.checked_mul(wave as u32)
				.ok_or_else(|| RecipeError::new("Nvidia wave register count overflows"))?;
			let observed = (registers as u32 / register_wave).min(threads as u32 / wave as u32);
			nvidia(cus as u32, wave as u32, workgroup as u32, block_lds as u32, sm_lds as u32, observed, resources) 		};
		let forward_geometry = geometry(forward_resource)?;
		let epoch_geometry = geometry(epoch_resource)?;
		for (kernel, geometry, action) in [
			(forward, forward_geometry, "forward occupancy"), 			(epoch, epoch_geometry, "epoch occupancy"), 		] {
			let mut active = 0;
			driver_status(Backend::Nvidia, occupancy(&mut active, kernel, geometry.block as i32, 0), action)?;
			require(active > 0, format!("Nvidia {action} has no resident workgroup"))?;
		}
		let cuda = Cuda { 			allocate: runtime.function(b"cuMemAlloc_v2\0")?,
			free: runtime.function(b"cuMemFree_v2\0")?, 			upload: runtime.function(b"cuMemcpyHtoD_v2\0")?,
			download: runtime.function(b"cuMemcpyDtoH_v2\0")?, 			synchronize: runtime.function(b"cuCtxSynchronize\0")?,
			launch: runtime.function(b"cuLaunchCooperativeKernel\0")?, 		};
		eprintln!("Nvidia forward block {} epoch block {}", forward_geometry.block, epoch_geometry.block);
		Ok(Gpu { 			backend: Backend::Nvidia, 			driver: Driver::Cuda(cuda),
			forward: Dispatch { kernel: Kernel::cuda(forward, FORWARD_ARGS), geometry: forward_geometry },
			epoch: Dispatch { kernel: Kernel::cuda(epoch, EPOCH_ARGS), geometry: epoch_geometry },
			dispatch: Mutex::new(()), 		}) 	} } #[cfg(any(feature = "amd", feature = "nvidia"))] #[link(name = "dl")]
unsafe extern "C" { 	fn dlopen(name: *const std::ffi::c_char, flags: i32) -> Ptr;
	fn dlsym(handle: Ptr, name: *const std::ffi::c_char) -> Ptr; }
unsafe extern "C" {
	fn signal(number: i32, handler: extern "C" fn(i32)) -> usize; }
fn distance(left: &[f64], right: &[f64]) -> f64 {
	left.iter().zip(right).map(|(a, b)| (a - b).powi(2)).sum()
} 	fn nearest(query: &[f64], state: &[f64], features: usize) -> (usize, f64) {
	state.chunks_exact(features)
		.enumerate()
		.map(|(index, row)| (index, distance(query, row)))
		.min_by(|left, right| left.1.total_cmp(&right.1))
		.unwrap_or((0, f64::INFINITY))
} 	fn graph_inputs(
	graph: &Graph,
	samples: &[f64],
	targets: &[f64],
	rows: usize,
	backend: Backend,
	tile: Tile,
) -> Result<Vec<f64>> {
	if graph.nodes.is_empty() {
		return Ok(samples[..rows * graph.output.elements()].to_vec());
	} 	let mut tape = DeviceTape::new(graph, samples, &targets[..rows], backend, tile)?;
	tape.forward()?;
	tape.predictions()
} 	fn fit_surrogate(
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
	let mut tape = DeviceTape::new(&graph, samples, targets, backend, config.tile)?;
	for step in 1..=config.surrogate_epochs {
		tape.epoch(step, config.surrogate_rate, mse, 0.0, config, false)?;
	} 	tape.weights(false)
} 	fn estimator_predict(
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
		} 	Operation::Knn(neighbors) => {
			let maximum = training_rows - usize::from(exclude_self);
			require(*neighbors != 0 && *neighbors <= maximum, "knn neighbor count is invalid")?;
			(1, *neighbors, training_rows * (data.features + 1))
		} 	_ => return Err(RecipeError::new("operation is not a supported estimator")),
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
			} 	for cluster in 0..argument {
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
					} 	}
			} 	}
	} else {
		fitted.extend_from_slice(&data.samples[..training_rows * data.features]);
		fitted.extend_from_slice(&data.targets[..training_rows]);
	} 	let inputs = &data.samples[training_rows * data.features..];
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
			} 	})
		.collect())
}
#[cfg(feature = "amd")]
const RDNA3_VGPRS_PER_SIMD: u32 = 1536;
#[cfg(feature = "amd")]
const RDNA3_VGPR_GRANULE: u32 = 24;
const DOUBLE_BUFFER_VALUES: u32 = 2;
#[derive(Clone, Copy)]
struct Resources {
	pub registers: u32, 	pub shared: u32, 	pub max_block: u32,
} 	#[derive(Clone, Copy)]
struct Geometry {
	pub groups: u32, 	pub block: u32,
} 	impl Geometry {
	pub fn threads(self, work: u32) -> Result<u32> {
		self.groups.min(work.div_ceil(self.block)).checked_mul(self.block).filter(|value| *value != 0)
			.ok_or_else(|| RecipeError::new("GPU launch size overflows"))
	} 	}
fn geometry(
	cus: u32, wave: u32, workgroup: u32, lds: u32, groups_per_cu: u32, resources: Resources,
) -> Result<Geometry> {
	require(wave != 0 && wave <= workgroup && wave <= resources.max_block, "GPU wave exceeds kernel workgroup")?;
	let waves = groups_per_cu.min(workgroup / wave).min(resources.max_block / wave);
	require(waves != 0, "GPU has no resident wave")?;
	let block = waves.checked_mul(wave).ok_or_else(|| RecipeError::new("GPU workgroup size overflows"))?;
	let tile = block.checked_mul(DOUBLE_BUFFER_VALUES * size_of::<f64>() as u32)
		.ok_or_else(|| RecipeError::new("GPU tile size overflows"))?;
	require(resources.shared.max(tile) <= lds, "GPU tile exceeds local memory")?;
	Ok(Geometry { groups: cus, block })
} 	#[cfg(feature = "amd")]
fn amd(
	cus: u32, wave: u32, workgroup: u32, lds: u32, waves_per_cu: u32, simds_per_cu: u32, resources: Resources,
) -> Result<Geometry> {
	let registers = resources.registers.div_ceil(RDNA3_VGPR_GRANULE) * RDNA3_VGPR_GRANULE;
	require(registers != 0, "AMD kernel register count is absent")?;
	let register_waves = RDNA3_VGPRS_PER_SIMD / registers * simds_per_cu;
	geometry(cus, wave, workgroup, lds, waves_per_cu.min(register_waves), resources)
} 	#[cfg(feature = "nvidia")]
fn nvidia(
	cus: u32, wave: u32, workgroup: u32, block_lds: u32, sm_lds: u32, waves_per_cu: u32, resources: Resources,
) -> Result<Geometry> {
	require(resources.registers != 0, "Nvidia kernel register count is absent")?;
	let tile = wave.checked_mul(DOUBLE_BUFFER_VALUES * size_of::<f64>() as u32)
		.ok_or_else(|| RecipeError::new("Nvidia tile size overflows"))?;
	require(resources.shared.max(tile) <= block_lds, "Nvidia tile exceeds workgroup shared memory")?;
	geometry(cus, wave, workgroup, sm_lds, waves_per_cu, resources)
}
pub trait IntoDataSources {
	fn into_data_sources(self) -> Vec<String>; } impl IntoDataSources for &str {
	fn into_data_sources(self) -> Vec<String> { 		vec![self.to_owned()] 	} } impl IntoDataSources for String {
	fn into_data_sources(self) -> Vec<String> { 		vec![self] 	} }
impl<T: Into<String>, const N: usize> IntoDataSources for [T; N] { 	fn into_data_sources(self) -> Vec<String> {
		self.into_iter().map(Into::into).collect() 	} } impl<T: Into<String>> IntoDataSources for Vec<T> {
	fn into_data_sources(self) -> Vec<String> { 		self.into_iter().map(Into::into).collect() 	} }
impl<T: Clone + Into<String>> IntoDataSources for &[T] { 	fn into_data_sources(self) -> Vec<String> {
	self.iter().cloned().map(Into::into).collect() 	} } impl Data {
	pub fn target(mut self, target: impl IntoDataSources) -> Self { 		self.target = target.into_data_sources(); 		self 	}
	pub fn r#in(mut self, names: impl IntoDataSources) -> Self {
		self.routes.push(Route { inputs: names.into_data_sources(), outputs: Vec::new() }); 		self 	}
	pub fn out(mut self, names: impl IntoDataSources) -> Self {
		self.routes.last_mut().unwrap_or_else(|| panic!(".out() requires a preceding .r#in()")).outputs =
			names.into_data_sources();
		self 	}
	pub fn exclude(mut self, names: impl IntoDataSources) -> Self { 		self.exclusions = names.into_data_sources(); 		self
	} 	pub fn set(mut self, source: impl Into<String>) -> Self { 		self.sources.push(source.into()); 		self 	}
	pub const fn norm(mut self, _: ZScore) -> Self { 		self.normalize = true; 		self 	}
	pub const fn split(mut self, fraction: f64) -> Self { 		self.split = fraction; 		self 	} } struct Prepared {
	samples: Vec<f64>, 	targets: Vec<f64>, 	rows: usize,
	features: usize, 	schema: String, } struct Table { 	name: String,
	headers: Vec<String>, 	rows: Vec<Vec<String>>, } enum FeatureType { 	Numeric(&'static str), 	Categorical(Vec<String>),
	Text(usize), }
fn prepare(data: &Data) -> Result<&Prepared> { 	match data.prepared.get_or_init(|| prepare_data(data)) {
		Ok(prepared) => Ok(prepared), 		Err(error) => Err(error.clone()), 	} }
fn prepare_data(data: &Data) -> Result<Prepared> { 	let mut paths = Vec::new(); 	for source in &data.sources {
		collect_files(&expand_home(source)?, &mut paths)?; 	}
	paths.sort();
	paths.dedup();
	let mut grouped = Vec::new(); 	for path in paths { 		let bytes = fs::read(&path)
			.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
		if path.extension().and_then(|value| value.to_str()).is_some_and(is_table) {
			let directory = path.parent().unwrap_or_else(|| Path::new("")).to_owned();
			grouped.push((directory, parse_table(&path, &bytes)?)); 		} 	}
	let mut tables = merge_captures(grouped, &data.target)?;
	tables = merge_partitions(tables, &data.target);
	require(!tables.is_empty(), "data source contains no supported table")?;
	let mut selected = Vec::new(); 	for name in &data.target {
		let mut matches = Vec::new(); 		for (table, value) in tables.iter().enumerate() {
			for (column, header) in value.headers.iter().enumerate() { 				let qualified = format!("{}.{}", value.name, header);
				let numbered = format!("col{}", column + 1);
				let qualified_numbered = format!("{}.{}", value.name, numbered);
				if name == header || name == &qualified || name == &numbered || name == &qualified_numbered {
					matches.push((table, column)); 				} 			} 		}
		require(matches.len() == 1, format!("target {name:?} must identify exactly one feature"))?;
		selected.push(matches[0]); 	}
	let table_index = selected.first().map_or(0, |target| target.0);
	let row_count = tables[table_index].rows.len();
	require(selected.iter().all(|target| tables[target.0].rows.len() == row_count), "target row counts differ")?;
	let fit_rows = ((row_count as f64) * data.split).floor().max(1.0) as usize;
	eprintln!("Feature name:                         Dtype:    Samples:"); 	for value in &tables {
		for (column, header) in value.headers.iter().enumerate() {
			let kind = infer_feature(value, column, fit_rows.min(value.rows.len())); 			let samples =
				value.rows.iter().filter(|row| row.get(column).is_some_and(|item| !item.is_empty())).count();
			eprintln!("{:<37} {:<9} {samples}", format!("{}.{}", value.name, header), kind.name()); 		} 	}
	let mut columns = Vec::new(); 	for (table, value) in tables.iter().enumerate() { 		if value.rows.len() == row_count {
			for (column, header) in value.headers.iter().enumerate() { 				let qualified = format!("{}.{}", value.name, header);
				let excluded = data.exclusions.iter().any(|name| name == header || name == &qualified);
				if !selected.contains(&(table, column)) && !excluded {
					columns.push((table, column, infer_feature(value, column, fit_rows))); 				} 			} 		} 	}
	let features = columns.iter().map(|column| column.2.width()).sum();
	require(features != 0, "dataset has no training features")?; 	let target_categories =
		selected.iter().map(|target| categories(&tables[target.0], target.1, fit_rows)).collect::<Vec<_>>();
	let mut samples = Vec::new();
	let mut targets = Vec::new(); 	for row in 0..row_count {
		let mut encoded = Vec::with_capacity(features); 		let valid = columns.iter().all(|column| {
			tables[column.0].rows[row].get(column.1).is_some_and(|value| encode(value, &column.2, &mut encoded)) 		});
		if valid && selected.is_empty() { 			samples.extend_from_slice(&encoded);
			targets.push(0.0); 		} else if valid { 			for (target, categories) in selected.iter().zip(&target_categories) {
				let value = tables[target.0].rows[row].get(target.1);
				let target = value.and_then(|value| value.parse::<f64>().ok()).or_else(|| {
					value.and_then(|value| categories.iter().position(|category| category == value)) 						.map(|value| value as f64)
				}); 				if let Some(target) = target 					&& target.is_finite() 				{
					samples.extend_from_slice(&encoded);
					targets.push(target); 				} 			} 		} 	}
	let rows = targets.len();
	require(rows != 0, "dataset has no complete training rows")?;
	shuffle(&mut samples, &mut targets, features)?; 	if data.normalize {
		normalize_samples(&mut samples, features, ((rows as f64) * data.split).floor() as usize)?; 	}
	let schema = columns 		.iter() 		.map(|column| {
			format!("{}.{}:{}", tables[column.0].name, tables[column.0].headers[column.1], column.2.width()) 		})
		.collect::<Vec<_>>() 		.join("|") + "->" 		+ &data.target.join("|");
	Ok(Prepared { samples, targets, rows, features, schema }) }
fn normalize_samples(samples: &mut [f64], features: usize, fit: usize) -> Result<()> {
	require(fit != 0, "split must retain normalization rows")?;
	let epsilon = number("normalization epsilon", env!("RECIPE_NORMALIZATION_EPSILON"))?; 	for column in 0..features {
		let mean = (0..fit).map(|row| samples[row * features + column]).sum::<f64>() / fit as f64; 		let variance =
			(0..fit).map(|row| (samples[row * features + column] - mean).powi(2)).sum::<f64>() / fit as f64;
		let scale = (variance + epsilon).sqrt(); 		for row in 0..samples.len() / features {
			samples[row * features + column] = (samples[row * features + column] - mean) / scale; 		} 	} 	Ok(()) }
fn is_table(extension: &str) -> bool { 	matches!(extension.to_ascii_lowercase().as_str(), "csv" | "tsv" | "txt") }
fn expand_home(source: &str) -> Result<PathBuf> { 	if source == "~" || source.starts_with("~/") {
		let home = std::env::var_os("HOME").ok_or_else(|| RecipeError::new("HOME is absent"))?;
		return Ok(PathBuf::from(home).join(source.trim_start_matches("~/"))); 	} 	Ok(PathBuf::from(source)) }
fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> { 	let metadata = fs::metadata(path)
		.map_err(|error| RecipeError::new(format!("cannot inspect {}: {error}", path.display())))?; 	if metadata.is_file() {
		files.push(path.to_owned());
		return Ok(()); 	} 	let mut children = fs::read_dir(path)
		.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?
		.collect::<std::io::Result<Vec<_>>>()
		.map_err(|error| RecipeError::new(format!("cannot read {}: {error}", path.display())))?;
	children.sort_by_key(fs::DirEntry::path); 	for child in children {
		collect_files(&child.path(), files)?; 	} 	Ok(()) }
fn target_column(table: &Table, name: &str) -> Option<usize> { 	table.headers.iter().enumerate()
		.position(|(column, header)| name == header || name == format!("col{}", column + 1)) }
fn merge_captures(tables: Vec<(PathBuf, Table)>, targets: &[String]) -> Result<Vec<Table>> {
	let mut groups = BTreeMap::<PathBuf, Vec<Table>>::new(); 	for (directory, table) in tables {
		groups.entry(directory).or_default().push(table); 	}
	let valid = |group: &[Table]| group.len() > 1 && targets.iter().all(|target| {
		group.iter().filter(|table| target_column(table, target).is_some()).count() == 1
			&& group.iter().find(|table| target_column(table, target).is_some()).is_some_and(|table| table.rows.len() == 1)
	});
	if targets.is_empty() || groups.values().filter(|group| valid(group)).count() < 2 {
		return Ok(groups.into_values().flatten().collect()); 	}
	let mut captures = groups.into_values().filter(|group| valid(group)).collect::<Vec<_>>();
	let key = |table: &Table| (table.headers.join("\0"), table.rows.len());
	for capture in &mut captures { 		capture.sort_by_key(&key); 	}
	let schemas = captures[0].iter().map(|table| (table.headers.clone(), table.rows.len())).collect::<Vec<_>>();
	for capture in &captures { 		require(capture.len() == schemas.len(), "capture table counts differ")?;
		require(capture.iter().zip(&schemas).all(|(table, schema)| {
			table.headers == schema.0 && table.rows.len() == schema.1
		}), "capture table schemas differ")?; 	}
	let names = (0..schemas.len()).map(|index| { 		let name = &captures[0][index].name;
		if captures.iter().all(|capture| capture[index].name == *name) { name.clone() } else { format!("table{}", index + 1) }
	}).collect::<Vec<_>>();
	let mut headers = Vec::new(); 	for (table, name) in captures[0].iter().zip(&names) {
		for row in 0..table.rows.len() { 			for header in &table.headers {
				if targets.contains(header) { 					headers.push(header.clone());
				} else if table.rows.len() == 1 { 					headers.push(format!("{name}.{header}"));
				} else { 					headers.push(format!("{name}.{header}.{}", row + 1)); 				} 			} 		} 	}
	let mut rows = Vec::with_capacity(captures.len()); 	for capture in captures {
		let row = capture.into_iter().flat_map(|table| table.rows.into_iter().flatten()).collect::<Vec<_>>();
		require(row.len() == headers.len(), "capture value width differs")?;
		rows.push(row); 	}
	Ok(vec![Table { name: "data".to_owned(), headers, rows }]) }
fn merge_partitions(mut tables: Vec<Table>, targets: &[String]) -> Vec<Table> {
	if targets.is_empty() || targets.iter().any(|target| target.contains('.')) { 		return tables; 	}
	let members = tables.iter().enumerate().filter_map(|(index, table)| {
		targets.iter().all(|target| target_column(table, target).is_some()).then_some(index) 	}).collect::<Vec<_>>();
	if members.len() < 2 { 		return tables 	} 	let mut headers = Vec::new();
	for &index in &members { 		for header in &tables[index].headers { 			if !headers.contains(header) {
				headers.push(header.clone()) 			} 		} 	} 	let mut rows = Vec::new();
	for index in members { 		let positions = tables[index].headers.iter()
			.map(|header| headers.iter().position(|value| value == header).unwrap()).collect::<Vec<_>>();
		for row in std::mem::take(&mut tables[index].rows) {
			let mut merged = std::iter::repeat_with(String::new).take(headers.len()).collect::<Vec<_>>();
			for (column, value) in row.into_iter().enumerate() { 				merged[positions[column]] = value; 			}
			rows.push(merged); 		} 	} 	vec![Table { name: "data".to_owned(), headers, rows }] }
fn parse_table(path: &Path, bytes: &[u8]) -> Result<Table> {
	let first = bytes.split(|byte| *byte == b'\n').next().unwrap_or_default();
	let delimiter = [b',', b';', b'\t'] 		.into_iter()
		.max_by_key(|delimiter| first.iter().filter(|byte| *byte == delimiter).count()) 		.unwrap_or(b',');
	let mut rows = records(bytes, delimiter)?;
	require(!rows.is_empty(), format!("dataset {} is empty", path.display()))?;
	let first = rows.remove(0);
	let headerless = first.iter().all(|value| value.parse::<f64>().is_ok()); 	let headers =
		if headerless { (1..=first.len()).map(|column| format!("col{column}")).collect() } else { first.clone() };
	if headerless { 		rows.insert(0, first); 	}
	let width = headers.len();
	rows.retain(|row| row.len() == width);
	let name = path.file_stem().and_then(|value| value.to_str()).unwrap_or("data").to_owned();
	Ok(Table { name, headers, rows }) } fn records(bytes: &[u8], delimiter: u8) -> Result<Vec<Vec<String>>> {
	let mut rows = Vec::new();
	let mut row = Vec::new();
	let mut field = Vec::new();
	let mut quoted = false;
	let mut index = 0; 	while index < bytes.len() {
		let byte = bytes[index]; 		if byte == b'"' { 			if quoted && bytes.get(index + 1) == Some(&b'"') {
				field.push(byte);
				index += 1; 			} else {
				quoted = !quoted; 			} 		} else if byte == delimiter && !quoted {
			row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?);
			field = Vec::new(); 		} else if byte == b'\n' && !quoted {
			let value = String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?;
			row.push(value.trim_end_matches('\r').to_owned());
			field = Vec::new(); 			if row.iter().any(|value| !value.is_empty()) {
				rows.push(row); 			}
			row = Vec::new(); 		} else {
			field.push(byte); 		}
		index += 1; 	}
	require(!quoted, "unterminated quoted feature")?; 	if !field.is_empty() || !row.is_empty() {
		row.push(String::from_utf8(field).map_err(|_| RecipeError::new("feature is not UTF-8"))?);
		rows.push(row); 	} 	Ok(rows) } fn categories(table: &Table, column: usize, rows: usize) -> Vec<String> { 	table.rows
		.iter() 		.take(rows) 		.filter_map(|row| row.get(column)) 		.filter(|value| !value.is_empty()) 		.cloned()
		.collect::<BTreeSet<_>>() 		.into_iter() 		.collect() }
fn infer_feature(table: &Table, column: usize, rows: usize) -> FeatureType {
	let values = table.rows.iter().take(rows).filter_map(|row| row.get(column)).filter(|value| !value.is_empty())
		.collect::<Vec<_>>();
	if !values.is_empty() && values.iter().all(|value| value.parse::<f64>().is_ok()) {
		return FeatureType::Numeric("f64"); 	}
	let categories = categories(table, column, rows); 	if categories.len() < values.len() {
		FeatureType::Categorical(categories) 	} else {
		FeatureType::Text(values.iter().map(|value| value.len()).max().unwrap_or(0)) 	} } impl FeatureType {
	const fn name(&self) -> &'static str { 		match self { 			Self::Numeric(name) => name,
			Self::Categorical(_) => "categoric", 			Self::Text(_) => "string", 		} 	} 	fn width(&self) -> usize { 		match self {
			Self::Numeric(_) => 1, 			Self::Categorical(values) => values.len(), 			Self::Text(width) => *width, 		} 	} }
fn encode(value: &str, kind: &FeatureType, output: &mut Vec<f64>) -> bool { 	if value.is_empty() {
		output.resize(output.len() + kind.width(), 0.0);
		return true;
	} 	match kind {
		FeatureType::Numeric(_) => value.parse::<f64>().is_ok_and(|value| { 			output.push(value); 			value.is_finite() 		}),
		FeatureType::Categorical(categories) => { 			let found = categories.iter().position(|category| category == value);
			output.extend((0..categories.len()).map(|index| f64::from(found == Some(index)))); 			found.is_some() 		}
		FeatureType::Text(width) => {
			output.extend(value.bytes().map(f64::from).chain(std::iter::repeat(0.0)).take(*width)); 			value.len() <= *width 		}
	} } fn shuffle(samples: &mut Vec<f64>, targets: &mut Vec<f64>, features: usize) -> Result<()> {
	let mut seed = env!("RECIPE_RANDOM_SEED") 		.parse::<u64>()
		.map_err(|error| RecipeError::new(format!("invalid random seed: {error}")))?;
	let mut order = (0..targets.len()).collect::<Vec<_>>(); 	for index in (1..order.len()).rev() {
		seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
		order.swap(index, (seed as usize) % (index + 1)); 	}
	let old_samples = std::mem::take(samples);
	let old_targets = std::mem::take(targets); 	for row in order {
		samples.extend_from_slice(&old_samples[row * features..(row + 1) * features]);
		targets.push(old_targets[row]); 	} 	Ok(()) }
pub struct Train { 	epochs: usize,
	learning_rate: f64, 	log_metrics: Vec<Metric>, 	stop: Option<f64>,
	resume: Option<String>, 	save: Option<String>, }
impl Train { 	pub const fn stop(mut self, value: f64) -> Self { 		self.stop = Some(value); 		self 	}
	pub const fn optimizer(self, _: Adamw) -> Self { 		self 	} 	pub const fn epochs(mut self, value: usize) -> Self {
		self.epochs = value; 		self 	} 	pub const fn lr(mut self, value: f64) -> Self {
		self.learning_rate = value; 		self 	}
	pub fn log<const N: usize>(mut self, metrics: [Metric; N]) -> Self {
		self.log_metrics = metrics.into(); 		self 	} 	pub fn save(mut self, path: impl Into<String>) -> Self {
		self.save = Some(path.into()); 		self 	} 	pub fn resume(mut self, path: impl Into<String>) -> Self {
		self.resume = Some(path.into()); 		self 	} 	pub fn run(&self, model: &Model, data: &Data) -> TrainingReport {
		SIGNAL.get_or_init(|| unsafe { signal(SIGINT, interrupt) }); 		if INTERRUPTED.load(Ordering::Acquire) {
			std::process::exit(INTERRUPTED_EXIT); 		} 		self.try_run(model, data).unwrap_or_else(|error| panic!("{error}")) 	}
	fn try_run(&self, model: &Model, data: &Data) -> Result<TrainingReport> { 		let backend = device_backend()?;
		let config = Config::load()?;
		require(model.downstream.is_none(), "model-valued loss requires .rat()")?;
		let prepared = prepare(data)?;
		let training_rows = ((prepared.rows as f64) * data.split).floor() as usize;
		require(training_rows != 0 && training_rows <= prepared.rows, "split must select training rows")?;
		let run = RUN.fetch_add(1, Ordering::Relaxed) + 1;
		let mut graph = compile(model, prepared, training_rows, backend, config)?;
		let output = graph.output.elements();
		require(output == 1, "model output width must be one")?; 		if let Some(path) = &self.resume {
			let mut stored = stored_graph(&graph, data);
			bundle::restore(path, &prepared.schema, std::slice::from_mut(&mut stored))?;
			graph = stored.graph;
			eprintln!("resumed: {path}"); 		}
		let samples = &prepared.samples[..training_rows * prepared.features];
		let targets = &prepared.targets[..training_rows];
		let mut tape = DeviceTape::new(&graph, samples, targets, backend, config.tile)?;
		let dispatched = tape.forward();
		self.finish_dispatch(dispatched, &graph, data, &prepared.schema, &tape, None)?;
		let initial_predictions = tape.predictions()?;
		let initial_loss = model_loss(&initial_predictions, targets, model.loss, config.activation[7]);
		let tolerance = self.stop.unwrap_or(0.0);
		require(tolerance.is_finite() && (0.0..=1.0).contains(&tolerance), "stop must be between zero and one")?;
		for epoch in 1..=self.epochs { 			let started = Instant::now();
			let dispatched = tape.epoch(epoch, self.learning_rate, model.loss, tolerance, config, false);
			let (loss, checkpoint) = self.finish_dispatch(dispatched, &graph, data, &prepared.schema, &tape, None)?;
			let predictions = tape.predictions()?;
			self.finish_dispatch(Ok(()), &graph, data, &prepared.schema, &tape, checkpoint.then_some(true))?;
			self.print(model, run, epoch, loss, targets, &predictions, started, checkpoint);
			self.finish_dispatch(Ok(()), &graph, data, &prepared.schema, &tape, None)?;
		} 		if self.stop.is_some() { 			tape.restore_best()?; 		}
		let dispatched = tape.forward();
		self.finish_dispatch(dispatched, &graph, data, &prepared.schema, &tape, None)?;
		let predictions = tape.predictions()?;
		let final_loss = model_loss(&predictions, targets, model.loss, config.activation[7]);
		let r2 = if training_rows == prepared.rows { 			coefficient(targets, &predictions) 		} else {
			graph.parameters = tape.weights(false)?;
			let start = training_rows * prepared.features;
			let validation_targets = &prepared.targets[training_rows..];
			let mut validation = DeviceTape::new(&graph, &prepared.samples[start..], validation_targets, backend, config.tile)?;
			validation.forward()?;
			coefficient(validation_targets, &validation.predictions()?) 		};
		self.finish_dispatch(Ok(()), &graph, data, &prepared.schema, &tape, Some(self.stop.is_some()))?;
		Ok(TrainingReport(initial_loss, final_loss, initial_predictions, predictions, r2)) 	}
	fn finish_dispatch<T>(
		&self,
		result: Result<T>,
		graph: &Graph,
		data: &Data,
		schema: &str,
		tape: &DeviceTape,
		save: Option<bool>,
	) -> Result<T> {
		let interrupted = INTERRUPTED.load(Ordering::Acquire);
		let save = if interrupted { Some(self.stop.is_some()) } else { save };
		if let Some(best) = save && let Some(path) = &self.save {
			save_graph(path, graph, data, schema, &tape.weights(best)?)?;
		} 	if interrupted {
			std::process::exit(INTERRUPTED_EXIT)
		} 	result
	} 	fn print( 		&self,
		model: &Model, 		run: u64, 		epoch: usize, 		loss: f64, 		targets: &[f64], 		predictions: &[f64], 		started: Instant,
		checkpoint: bool, 	) { 		if self.log_metrics.is_empty() { 			return; 		}
		let topology = model.description(&self.log_metrics);
		let r2 = coefficient(targets, predictions);
		let time = started.elapsed().as_secs_f64() * 1000.0;
		let mut values = Vec::new();
		let mut topology_printed = false; 		for metric in &self.log_metrics { 			let value = match metric.0 {
				0 => format!("run \x1b[38\x3b2\x3b242\x3b40\x3b60m{run:>5}\x1b[0m"),
				1 => format!("{} \x1b[38\x3b2\x3b0\x3b174\x3b107m{loss:.4}\x1b[0m", model.loss.name()),
				2 => format!("r2 \x1b[38\x3b2\x3b39\x3b125\x3b255m{r2:>7.4}\x1b[0m"),
				3 => format!("time \x1b[38\x3b2\x3b255\x3b194\x3b0m{time:>9.3} ms\x1b[0m"),
				4 => format!("epoch \x1b[38\x3b2\x3b135\x3b90\x3b251m{epoch}\x1b[0m"),
				5..=7 if !topology_printed && !topology.is_empty() => { 					topology_printed = true; 					topology.clone() 				}
				5..=7 => continue, 				_ => unreachable!(), 			};
			values.push(value); 		} 		if checkpoint && self.stop.is_some() {
			values.push("\x1b[1\x3b32m← checkpoint\x1b[0m".to_owned()); 		}
		eprintln!("{}", values.join("  ")); 	} }
pub struct TrainingReport(f64, f64, Vec<f64>, Vec<f64>, f64); impl TrainingReport {
	pub const fn initial_loss(&self) -> f64 { 		self.0 	} 	pub const fn final_loss(&self) -> f64 { 		self.1 	}
	pub fn initial_predictions(&self) -> &[f64] { 		&self.2 	} 	pub fn predictions(&self) -> &[f64] { 		&self.3 	}
	pub const fn r2(&self) -> f64 { 		self.4 	} }
fn model_loss(predictions: &[f64], targets: &[f64], loss: LossFunction, threshold: f64) -> f64 {
	let values = predictions.iter().zip(targets);
	let mut result = values.map(|(prediction, target)| loss.value(*prediction, *target, threshold)).sum::<f64>()
		/ targets.len() as f64; 	if loss.0 == 1 {
		result = result.sqrt(); 	} 	result } fn coefficient(targets: &[f64], predictions: &[f64]) -> f64 {
	let mean = targets.iter().sum::<f64>() / targets.len() as f64;
	let residual = targets.iter().zip(predictions).map(|(target, value)| (target - value).powi(2)).sum::<f64>();
	let total = targets.iter().map(|target| (target - mean).powi(2)).sum::<f64>();
	if total == 0.0 { 0.0 } else { 1.0 - residual / total } }
use std::{
	io::{self, BufRead, BufReader, Write},
	process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
fn process_io<T>(action: &str, result: io::Result<T>) -> Result<T> {
	result.map_err(|error| RecipeError::new(format!("cannot {action} RAT command: {error}"))) }
struct Frame(Vec<(String, f64)>); impl Frame { 	fn values(&self, names: &[String]) -> Result<Vec<f64>> {
	names.iter().map(|name| self.0.iter().find(|value| value.0 == *name).map(|value| value.1)
		.ok_or_else(|| RecipeError::new(format!("RAT value {name:?} is absent")))).collect() 	} }
struct Process { 	child: Child, 	input: Option<ChildStdin>, 	output: BufReader<ChildStdout>, } impl Process {
	fn spawn(command: &str) -> Result<Self> { 		require(!command.trim().is_empty(), ".every() requires a command")?;
		let mut child = Command::new(command).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn()
			.map_err(|error| RecipeError::new(format!("cannot start {command:?}: {error}")))?;
		let input = child.stdin.take().ok_or_else(|| RecipeError::new("RAT command stdin is absent"))?;
		let output = child.stdout.take().ok_or_else(|| RecipeError::new("RAT command stdout is absent"))?;
		Ok(Self { child, input: Some(input), output: BufReader::new(output) }) 	}
	fn read(&mut self) -> Result<Frame> { 		let mut values = Vec::new();
		loop { 			let mut line = String::new();
			let bytes = process_io("read", self.output.read_line(&mut line))?;
			require(bytes != 0, "RAT command exited before a blank-line frame terminator")?;
			let line = line.trim(); 			if line.is_empty() {
				require(!values.is_empty(), "RAT command returned an empty frame")?;
				return Ok(Frame(values)) 			}
			let Some((name, value)) = line.split_once(char::is_whitespace) else { 				continue 			};
			let value = value.trim().parse::<f64>()
				.map_err(|error| RecipeError::new(format!("RAT value {name:?} is invalid: {error}")))?;
			require(value.is_finite(), format!("RAT value {name:?} must be finite"))?;
			require(!values.iter().any(|item: &(String, f64)| item.0 == name),
				format!("RAT value {name:?} is duplicated"))?;
			values.push((name.to_owned(), value)); 		} 	}
	fn write(&mut self, names: &[String], values: &[f64]) -> Result<()> {
		require(names.len() == values.len(), "RAT proposal has the wrong width")?;
		let input = self.input.as_mut().ok_or_else(|| RecipeError::new("RAT command stdin is closed"))?;
		process_io("write", writeln!(input, "proposal"))?; 		for (name, value) in names.iter().zip(values) {
			process_io("write", writeln!(input, "    {name} {value}"))?; 		}
		process_io("write", writeln!(input))?; 		process_io("flush", input.flush()) 	} }
impl Drop for Process { 	fn drop(&mut self) { 		drop(self.input.take());
		let _ = self.child.wait(); 	} }
struct State { 	stored: bundle::StoredGraph, 	tape: DeviceTape, 	proposals: Vec<(usize, usize)>,
	proposal_names: Vec<String>, 	targets: Vec<f64>, 	rows: usize, 	next: usize, 	step: usize, 	schema: String, }
pub struct RatTrain<const N: usize> { 	train: Train, 	models: [Model; N], 	command: Option<String>,
	process: Option<Process>, 	context: Option<Frame>, 	state: Option<State>, }
pub struct RatReport { 	proposal: Vec<f64>, 	prediction: Vec<f64>, 	measurement: Vec<f64>, } impl RatReport {
	pub fn proposal(&self) -> &[f64] { 		&self.proposal 	}
	pub fn prediction(&self) -> &[f64] { 		&self.prediction 	}
	pub fn measurement(&self) -> &[f64] { 		&self.measurement 	} }
fn rat<const N: usize>(train: Train, models: [Model; N]) -> RatTrain<N> {
	RatTrain { train, models, command: None, process: None, context: None, state: None } }
impl Train { 	pub fn rat(self, proposer: Model, predictor: Model) -> RatTrain<2> { 		rat(self, [proposer, predictor]) 	}
	pub fn rats<const N: usize>(self, models: [Model; N]) -> RatTrain<N> { 		rat(self, models) 	} }
fn rat_schema(data: &Data) -> String { 	data.routes.iter()
	.map(|route| format!("{}->{}", route.inputs.join("|"), route.outputs.join("|")))
	.chain(std::iter::once(format!("target->{}", data.target.join("|")))).collect::<Vec<_>>().join("/") }
fn append_model(
	graph: &mut Graph, model: &Model, features: usize, outputs: usize, backend: Backend, config: Config, schema: &str,
) -> Result<i32> { 	let rows = config.rat_batch;
	let prepared = Prepared { 		samples: vec![0.0; rows * features],
		targets: vec![0.0; rows],
		rows, 		features, 		schema: schema.to_owned(), 	};
	let part = compile_output(model, &prepared, rows, backend, config, outputs)?;
	append_graph(graph, part) }
fn build<const N: usize>(
	models: &[Model; N], train: &Train, data: &Data, backend: Backend, config: Config,
) -> Result<State> {
	require(N >= 2, "RAT requires an intermediate model and a predictor")?;
	require(data.routes.len() + 1 == N, "RAT requires one .r#in().out() pair per intermediate model")?;
	require(!data.target.is_empty(), "RAT requires .target()")?;
	let input_names = data.routes[0].inputs.clone();
	require(!input_names.is_empty(), "RAT requires an initial input")?;
	let input = Shape { channels: input_names.len(), length: 1 };
	let mut graph = Graph { 		nodes: Vec::new(), 		parameters: Vec::new(), 		frozen: Vec::new(),
		programs: Vec::new(), 		input, 		output: input, 		source: -1, 	};
	let mut fields = Vec::new(); 	for (index, name) in input_names.iter().cloned().enumerate() {
		require(!fields.iter().any(|value: &(String, Field)| value.0 == name), "RAT input names must be unique")?;
		fields.push((name, Field { source: -1, stride: input_names.len(), index })); 	}
	let schema = rat_schema(data);
	let mut proposals = Vec::new();
	let mut proposal_names = Vec::new();
	for (index, route) in data.routes.iter().enumerate() { 		if let Some(downstream) = &models[index].downstream {
			require(downstream == &models[index + 1].blocks, "model-valued loss must name the next RAT model")?; 		}
		require(!route.inputs.is_empty() && !route.outputs.is_empty(), "RAT route names must not be empty")?;
		route_graph(&mut graph, &route.inputs, &fields)?;
		let source = append_model(
			&mut graph, &models[index], route.inputs.len(), route.outputs.len(), backend, config, &schema,
		)?;
		proposals.push((source as usize, route.outputs.len()));
		for (field_index, name) in route.outputs.iter().cloned().enumerate() {
			require(!fields.iter().any(|value| value.0 == name), format!("RAT output {name:?} is duplicated"))?;
			fields.push((name.clone(), Field { source, stride: route.outputs.len(), index: field_index }));
			proposal_names.push(name); 		} 	}
	require(proposal_names.len() >= 3, "RAT requires M, N, and K proposal outputs")?;
	let route = data.routes.last().ok_or_else(|| RecipeError::new("RAT route is absent"))?;
	require(models[N - 1].downstream.is_none(), "the final RAT model requires a scalar loss")?;
	let mut predictor_inputs = route.inputs.clone();
	predictor_inputs.extend(route.outputs.iter().cloned());
	route_graph(&mut graph, &predictor_inputs, &fields)?;
	append_model(&mut graph, &models[N - 1], predictor_inputs.len(), data.target.len(), backend, config, &schema)?;
	let mut stored = bundle::StoredGraph { graph, inputs: input_names, outputs: data.target.clone() };
	if let Some(path) = &train.resume { 		bundle::restore(path, &schema, std::slice::from_mut(&mut stored))?;
		eprintln!("resumed: {path}"); 	}
	let sample_count = config.rat_batch * stored.inputs.len();
	let target_count = config.rat_batch * stored.outputs.len();
	let tape = DeviceTape::new(
		&stored.graph,
		&vec![0.0; sample_count],
		&vec![0.0; target_count], backend, config.tile,
	)?;
	Ok(State { 		stored, 		tape, 		proposals, 		proposal_names, 		targets: vec![0.0; target_count],
		rows: 0, 		next: 0, 		step: 0, 		schema, 	}) }
impl State { 	fn row(&mut self) -> Result<usize> { 		let row = self.next;
		self.next = (self.next + 1) % self.tape.capacity;
		self.rows = (self.rows + 1).min(self.tape.capacity);
		self.tape.activate(self.rows)?; 		Ok(row) 	}
	fn proposal(&self, row: usize) -> Result<Vec<f64>> { 		let mut values = Vec::new();
		for &(node, width) in &self.proposals { 			values.extend(self.tape.node_row(node, row, width)?); 		}
		Ok(values) 	}
	fn capture(&mut self) -> Result<()> { 		self.stored.graph.parameters = self.tape.weights(false)?; 		Ok(()) 	} }
fn train_rat(
	state: &mut State, train: &Train, predictor: &Model, row: usize, measurement: &[f64], config: Config,
) -> Result<Vec<f64>> { 	state.tape.write_targets(row, measurement)?;
	let width = state.stored.outputs.len();
	state.targets[row * width..(row + 1) * width].copy_from_slice(measurement);
	let run = RUN.fetch_add(1, Ordering::Relaxed) + 1;
	let mut current = Vec::new();
	for epoch in 1..=train.epochs { 		let started = Instant::now();
		state.step += 1;
		let (loss, _) = state.tape.epoch(state.step, train.learning_rate, predictor.loss, 0.0, config, false)?;
		let predictions = state.tape.predictions()?;
		let target_count = state.rows * width;
		train.print(predictor, run, epoch, loss, &state.targets[..target_count], &predictions, started, false);
		current = predictions[row * width..(row + 1) * width].to_vec(); 	}
	Ok(current) }
impl<const N: usize> RatTrain<N> { 	pub fn every(mut self, command: impl Into<String>) -> Self {
		self.command = Some(command.into()); 		self 	}
	pub fn save(mut self, path: impl Into<String>) -> Self { 		self.train.save = Some(path.into()); 		self 	}
	pub fn resume(mut self, path: impl Into<String>) -> Self { 		self.train.resume = Some(path.into()); 		self 	}
	fn process(&mut self) -> Result<&mut Process> { 		if self.process.is_none() {
			let command = self.command.as_deref().ok_or_else(|| RecipeError::new("RAT requires .every()"))?;
			self.process = Some(Process::spawn(command)?); 		}
		self.process.as_mut().ok_or_else(|| RecipeError::new("RAT command is absent")) 	}
	fn check_interrupt(&mut self, state: Option<&mut State>) -> Result<()> {
		if !INTERRUPTED.load(Ordering::Acquire) { 			return Ok(()) 		}
		if let Some(state) = state { 			state.capture()?; 			if let Some(path) = &self.train.save {
				bundle::save(path, &state.schema, std::slice::from_ref(&state.stored))?; 			} 		}
		drop(self.process.take()); 		std::process::exit(INTERRUPTED_EXIT) 	}
	pub fn run(&mut self, data: &Data) -> RatReport {
		SIGNAL.get_or_init(|| unsafe { signal(SIGINT, interrupt) });
		self.try_run(data).unwrap_or_else(|error| panic!("{error}")) 	}
	fn try_run(&mut self, data: &Data) -> Result<RatReport> {
		if INTERRUPTED.load(Ordering::Acquire) { 			let mut state = self.state.take();
			self.check_interrupt(state.as_mut())?; 		}
		let context = match self.context.take() { 			Some(context) => context, 			None => self.process()?.read()?, 		};
		self.check_interrupt(None)?;
		let backend = device_backend()?;
		let config = Config::load()?;
		if self.state.is_none() { 			self.state = Some(build(&self.models, &self.train, data, backend, config)?); 		}
		let mut state = self.state.take().ok_or_else(|| RecipeError::new("RAT state is absent"))?;
		let row = state.row()?;
		state.tape.write_samples(row, &context.values(&state.stored.inputs)?)?;
		state.tape.forward()?;
		let mut proposal = state.proposal(row)?;
		state.tape.tile = Tile::proposed(&mut proposal, config.tile_limit)?;
		let written = self.process()?.write(&state.proposal_names, &proposal);
		self.check_interrupt(Some(&mut state))?;
		written?;
		let result = self.process()?.read();
		self.check_interrupt(Some(&mut state))?;
		let result = result?;
		let measurement = result.values(&data.target)?;
		let prediction = train_rat(&mut state, &self.train, &self.models[N - 1], row, &measurement, config)?;
		self.check_interrupt(Some(&mut state))?;
		self.context = Some(result);
		if let Some(path) = &self.train.save { 			state.capture()?;
			bundle::save(path, &state.schema, std::slice::from_ref(&state.stored))?; 		}
		self.state = Some(state); 		Ok(RatReport { proposal, prediction, measurement }) 	} }
