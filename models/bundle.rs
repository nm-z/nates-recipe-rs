use super::*;
use std::{collections::BTreeMap, io::Write as _, str::FromStr};

#[derive(Clone)]
pub(super) struct StoredGraph {
	pub graph: Graph,
	pub inputs: Vec<String>,
	pub outputs: Vec<String>,
}

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
}

impl Builder {
	fn new() -> Self {
		Self {
			inputs: Vec::new(),
			outputs: Vec::new(),
			input: None,
			output: None,
			nodes: Vec::new(),
			arguments: 0,
			parameters: Vec::new(),
			frozen: Vec::new(),
			programs: Vec::new(),
		}
	}

	fn finish(self) -> Result<StoredGraph> {
		let input = self.input.ok_or_else(|| RecipeError::new("model graph has no input shape"))?;
		let output = self.output.ok_or_else(|| RecipeError::new("model graph has no output shape"))?;
		require(!self.nodes.is_empty(), "model graph has no nodes")?;
		require(self.arguments == self.nodes.len(), "model graph node arguments are incomplete")?;
		require(self.parameters.len() == self.frozen.len(), "model graph frozen weights are incomplete")?;
		require(self.inputs.len() == input.elements(), "model graph input schema has the wrong width")?;
		require(self.outputs.len() == output.elements(), "model graph output schema has the wrong width")?;
		let source = self.nodes.len() as i32 - 1;
		Ok(StoredGraph {
			graph: Graph {
				nodes: self.nodes,
				parameters: self.parameters,
				frozen: self.frozen,
				programs: self.programs,
				input,
				output,
				source,
			},
			inputs: self.inputs,
			outputs: self.outputs,
		})
	}
}

fn parse_values<T>(text: &str, role: &str) -> Result<Vec<T>>
where
	T: FromStr,
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
		8 => Ok(Primitive::Normalize),
		_ => Err(RecipeError::new(format!("model graph primitive {value} is invalid"))),
	}
}

fn node(values: &[i32]) -> Result<Node> {
	require(values.len() == 11, "model graph node descriptor has the wrong width")?;
	Ok(Node {
		op: primitive(values[0])?,
		source: values[1],
		second: values[2],
		input: Shape { channels: values[3] as usize, length: values[4] as usize },
		output: Shape { channels: values[5] as usize, length: values[6] as usize },
		offset: values[7] as usize,
		parameters: values[8] as usize,
		argument: [0.0; 9],
		program_offset: values[9] as usize,
		program_count: values[10] as usize,
	})
}

pub(super) fn load(path: &str) -> Result<(String, Vec<StoredGraph>)> {
	require(path.ends_with(".ogdl"), "model path requires .ogdl")?;
	let document = fs::read_to_string(path).map_err(|error| RecipeError::new(format!("cannot read {path}: {error}")))?;
	let mut schema = String::new();
	let mut builders = Vec::new();
	let mut current: Option<Builder> = None;
	for line in document.lines() {
		let line = line.trim();
		if line == "recipe-model" {
			continue;
		}
		if line == "graph" {
			if let Some(builder) = current.take() {
				builders.push(builder.finish()?);
			}
			current = Some(Builder::new());
			continue;
		}
		if let Some(value) = line.strip_prefix("schema ") {
			schema = value.to_owned();
			continue;
		}
		let builder = current.as_mut().ok_or_else(|| RecipeError::new("model value precedes graph"))?;
		if let Some(value) = line.strip_prefix("in ") {
			builder.inputs.push(value.to_owned());
		} else if let Some(value) = line.strip_prefix("out ") {
			builder.outputs.push(value.to_owned());
		} else if let Some(value) = line.strip_prefix("shape ") {
			let shape = parse_values::<usize>(value, "model shape")?;
			require(shape.len() == 4, "model graph shape has the wrong width")?;
			builder.input = Some(Shape { channels: shape[0], length: shape[1] });
			builder.output = Some(Shape { channels: shape[2], length: shape[3] });
		} else if let Some(value) = line.strip_prefix("node ") {
			builder.nodes.push(node(&parse_values::<i32>(value, "node descriptor")?)?);
		} else if let Some(value) = line.strip_prefix("arguments ") {
			let values = parse_values::<f64>(value, "node argument")?;
			require(values.len() == 9, "model graph node argument has the wrong width")?;
			builder
				.nodes
				.last_mut()
				.ok_or_else(|| RecipeError::new("model graph argument precedes node"))?
				.argument
				.copy_from_slice(&values);
			builder.arguments += 1;
		} else if let Some(value) = line.strip_prefix("programs") {
			builder.programs = parse_values(value, "scalar program")?;
		} else if let Some(value) = line.strip_prefix("weights") {
			builder.parameters = parse_values(value, "weight")?;
		} else if let Some(value) = line.strip_prefix("frozen") {
			builder.frozen = parse_values(value, "frozen weight")?;
		} else if !line.is_empty() {
			return Err(RecipeError::new(format!("model graph value is invalid: {line}")));
		}
	}
	if let Some(builder) = current {
		builders.push(builder.finish()?);
	}
	require(!builders.is_empty(), "model has no graphs")?;
	Ok((schema, builders))
}

fn join<T: ToString>(values: &[T]) -> String {
	values.iter().map(ToString::to_string).collect::<Vec<_>>().join(" ")
}

pub(super) fn save(path: &str, schema: &str, graphs: &[StoredGraph]) -> Result<()> {
	require(path.ends_with(".ogdl"), "save requires an .ogdl model")?;
	require(!graphs.is_empty(), "model bundle has no graphs")?;
	let mut document = format!("recipe-model\n    schema {schema}\n");
	for stored in graphs {
		document.push_str("    graph\n");
		for name in &stored.inputs {
			document.push_str(&format!("        in {name}\n"));
		}
		for name in &stored.outputs {
			document.push_str(&format!("        out {name}\n"));
		}
		let graph = &stored.graph;
		document.push_str(&format!(
			"        shape {} {} {} {}\n",
			graph.input.channels, graph.input.length, graph.output.channels, graph.output.length
		));
		for node in &graph.nodes {
			let descriptor = [
				node.op as i32,
				node.source,
				node.second,
				node.input.channels as i32,
				node.input.length as i32,
				node.output.channels as i32,
				node.output.length as i32,
				node.offset as i32,
				node.parameters as i32,
				node.program_offset as i32,
				node.program_count as i32,
			];
			document.push_str(&format!("        node {}\n", join(&descriptor)));
			document.push_str(&format!("            arguments {}\n", join(&node.argument)));
		}
		document.push_str(&format!("        programs {}\n", join(&graph.programs)));
		document.push_str(&format!("        weights {}\n", join(&graph.parameters)));
		document.push_str(&format!("        frozen {}\n", join(&graph.frozen)));
	}
	fs::write(path, document).map_err(|error| RecipeError::new(format!("cannot write {path}: {error}")))?;
	eprintln!("saved: {path}");
	Ok(())
}

fn same_node(left: &Node, right: &Node) -> bool {
	left.op == right.op
		&& left.source == right.source
		&& left.second == right.second
		&& left.input == right.input
		&& left.output == right.output
		&& left.offset == right.offset
		&& left.parameters == right.parameters
		&& left.argument.map(f64::to_bits) == right.argument.map(f64::to_bits)
		&& left.program_offset == right.program_offset
		&& left.program_count == right.program_count
}

fn same_graph(left: &StoredGraph, right: &StoredGraph) -> bool {
	left.inputs == right.inputs
		&& left.outputs == right.outputs
		&& left.graph.input == right.graph.input
		&& left.graph.output == right.graph.output
		&& left.graph.frozen == right.graph.frozen
		&& left.graph.programs == right.graph.programs
		&& left.graph.parameters.len() == right.graph.parameters.len()
		&& left.graph.nodes.len() == right.graph.nodes.len()
		&& left.graph.nodes.iter().zip(&right.graph.nodes).all(|(a, b)| same_node(a, b))
}

pub(super) fn restore(path: &str, schema: &str, graphs: &mut [StoredGraph]) -> Result<()> {
	if !fs::exists(path).map_err(|error| RecipeError::new(format!("cannot inspect {path}: {error}")))? {
		return save(path, schema, graphs);
	}
	let (stored_schema, stored) = load(path)?;
	let matches = stored_schema == schema
		&& stored.len() == graphs.len()
		&& stored.iter().zip(graphs.iter()).all(|(a, b)| same_graph(a, b));
	if matches {
		for (current, saved) in graphs.iter_mut().zip(stored) {
			current.graph.parameters = saved.graph.parameters;
		}
		return Ok(());
	}
	eprint!("mismatch: overwrite {path}? Y/n ");
	std::io::stderr()
		.flush()
		.map_err(|error| RecipeError::new(format!("cannot prompt: {error}")))?;
	let mut answer = String::new();
	std::io::stdin()
		.read_line(&mut answer)
		.map_err(|error| RecipeError::new(format!("cannot read answer: {error}")))?;
	require(answer.trim().is_empty() || answer.trim().eq_ignore_ascii_case("y"), "model mismatch not overwritten")?;
	save(path, schema, graphs)
}

pub(super) fn infer(path: &str, input: &[f64]) -> Result<Vec<f64>> {
	let (_, graphs) = load(path)?;
	let first = graphs.first().ok_or_else(|| RecipeError::new("model has no graph"))?;
	require(input.len() == first.inputs.len(), "model input has the wrong width")?;
	let mut values = first.inputs.iter().cloned().zip(input.iter().copied()).collect::<BTreeMap<_, _>>();
	let backend = device_backend()?;
	let mut result = Vec::new();
	for stored in graphs {
		let samples = stored
			.inputs
			.iter()
			.map(|name| values.get(name).copied().ok_or_else(|| RecipeError::new(format!("input {name:?} is absent"))))
			.collect::<Result<Vec<_>>>()?;
		let mut tape = DeviceTape::new(&stored.graph, &samples, &[], backend)?;
		tape.forward()?;
		result = tape.predictions()?;
		require(result.len() == stored.outputs.len(), "model output has the wrong width")?;
		for (name, value) in stored.outputs.iter().cloned().zip(result.iter().copied()) {
			values.insert(name, value);
		}
	}
	Ok(result)
}
