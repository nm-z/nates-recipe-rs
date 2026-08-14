use super::*;
use std::{
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
	process::Command,
	sync::atomic::{AtomicUsize, Ordering},
};

#[derive(Clone)]
pub(crate) struct NativeLayout {
	pub values: Vec<usize>,
	pub contexts: Vec<usize>,
	pub adjoints: Vec<usize>,
	pub values_bytes: usize,
	pub contexts_bytes: usize,
	pub adjoints_bytes: usize,
}

fn align(value: usize, boundary: usize) -> Result<usize> {
	let boundary = boundary.max(1);
	let remainder = value % boundary;
	if remainder == 0 { Ok(value) } else { checked_add(value, boundary - remainder, "native arena alignment") }
}

impl NativeLayout {
	pub(crate) fn for_graph(graph: &Graph, rows: usize, precision: Compute) -> Result<Self> {
		let element = precision.bytes();
		let mut values = Vec::with_capacity(graph.nodes.len());
		let mut contexts = Vec::with_capacity(graph.nodes.len());
		let mut adjoints = Vec::with_capacity(graph.nodes.len());
		let (mut value_offset, mut context_offset, mut adjoint_offset) = (0, 0, 0);
		for node in &graph.nodes {
			value_offset = align(value_offset, element.max(8))?;
			context_offset = align(context_offset, element.max(8))?;
			adjoint_offset = align(adjoint_offset, element.max(8))?;
			values.push(value_offset);
			contexts.push(context_offset);
			adjoints.push(adjoint_offset);
			value_offset = checked_add(value_offset, graph_rows_buffer(node.output, rows, element)?, "model value arena")?;
			context_offset = checked_add(context_offset, node_context(node, rows, element)?, "model context arena")?;
			adjoint_offset = checked_add(adjoint_offset, graph_rows_buffer(node.output, rows, element)?, "model adjoint arena")?;
		}
		Ok(Self { values, contexts, adjoints, values_bytes: value_offset.max(element), contexts_bytes: context_offset.max(element), adjoints_bytes: adjoint_offset.max(element) })
	}
}

#[derive(Clone)]
struct NodePlan {
	node: Node,
	value: usize,
	context: usize,
	adjoint: usize,
	stored: Option<StoredWeight>,
	storage_offset: usize,
	codebook_offset: usize,
}

#[derive(Clone)]
pub(crate) struct NativeModelIr {
	graph: Graph,
	layout: NativeLayout,
	precision: Compute,
	rows: usize,
	plans: Vec<NodePlan>,
	storage_bytes: usize,
	codebook_values: usize,
}

impl NativeModelIr {
	pub(crate) fn from_graph(graph: &Graph, rows: usize, precision: Compute) -> Result<Self> {
		require(rows != 0, "native model rows must be positive")?;
		let layout = NativeLayout::for_graph(graph, rows, precision)?;
		let mut plans = Vec::with_capacity(graph.nodes.len());
		let mut storage_bytes = 0usize;
		let mut codebook_values = 0usize;
		for (index, node) in graph.nodes.iter().cloned().enumerate() {
			let id = || node.identity(index);
			require(node.source >= -1 && node.source < index as i32, format!("{} has invalid source node {}", id(), node.source))?;
			require(node.second >= -2 && node.second < index as i32, format!("{} has invalid second source node {}", id(), node.second))?;
			require(node.offset.checked_add(node.parameters).is_some_and(|end| end <= graph.parameters.len()), format!("{} parameter range exceeds {} values", id(), graph.parameters.len()))?;
			let width = if node.op == Primitive::Predictor { 2 } else { 3 };
			let program_width = node.program_count.checked_mul(width).ok_or_else(|| RecipeError::new(format!("{} program length overflows", id())))?;
			require(node.program_offset.checked_add(program_width).is_some_and(|end| end <= graph.programs.len()), format!("{} program range exceeds {} values", id(), graph.programs.len()))?;
			let stored = graph.stored.get(index).cloned().unwrap_or(None);
			if let Some(weight) = &stored {
				require(weight.count == node.parameters, format!("{} stored weight count {} does not match parameter count {}", id(), weight.count, node.parameters))?;
			}
			let storage_offset = storage_bytes;
			let codebook_offset = codebook_values;
			if let Some(weight) = &stored {
				storage_bytes = checked_add(storage_bytes, weight.bytes.len(), "native storage arena")?;
				codebook_values = checked_add(codebook_values, weight.codebook.len(), "native codebook arena")?;
			}
			plans.push(NodePlan { node, value: layout.values[index], context: layout.contexts[index], adjoint: layout.adjoints[index], stored, storage_offset, codebook_offset });
		}
		Ok(Self { graph: graph.clone(), layout, precision, rows, plans, storage_bytes, codebook_values })
	}
	pub(crate) fn layout(&self) -> &NativeLayout { &self.layout }
}

fn precision_source(precision: Compute) -> (&'static str, &'static str) {
	match precision {
		Compute::F(_) => ("default", "recipe-cpu"),
		Compute::Fp(format) if format == FloatFormat::FP64 => ("default", "recipe-cpu"),
		Compute::Fp(format) if format == FloatFormat::FP32 => ("-f32", "recipe-cpu"),
		Compute::Fp(format) if format == FloatFormat::FP16 => ("-f16", "recipe-cpu"),
		Compute::Fp(format) if format == FloatFormat::FP8 => ("-f8", "recipe-cpu"),
		Compute::Fp(_) => ("-f8", "recipe-cpu"),
		Compute::Bf(_) => ("-bf16", "recipe-cpu"),
		Compute::Tf(_) => ("-tf32", "recipe-cpu"),
		Compute::Int(IntFormat { bits: 8 }) => ("-int8", "recipe-cpu"),
		Compute::Int(IntFormat { bits: 4 }) => ("-int4", "recipe-cpu"),
		Compute::Int(IntFormat { bits: 1 }) => ("-int1", "recipe-cpu"),
		Compute::Int(_) => ("-int8", "recipe-cpu"),
	}
}

fn template_path(mapping: &str, suffix: &str) -> Result<PathBuf> {
	let key = if suffix.is_empty() { "default" } else { suffix };
	let path = mapping.split(';').find_map(|entry| entry.split_once('=').filter(|(name, _)| *name == key).map(|(_, path)| PathBuf::from(path))).ok_or_else(|| RecipeError::new(format!("native LLVM template {key:?} is absent")))?;
	Ok(path)
}

fn backend_template(backend: Backend, precision: Compute) -> Result<String> {
	let (suffix, _) = precision_source(precision);
	let mapping = match backend {
		Backend::Cpu => option_env!("RECIPE_CPU_IR").ok_or_else(|| RecipeError::new("CPU native LLVM templates are unavailable"))?,
		Backend::Amd => option_env!("RECIPE_AMD_IR").ok_or_else(|| RecipeError::new("AMD native LLVM templates are unavailable"))?,
		Backend::Nvidia => option_env!("RECIPE_NV_IR").ok_or_else(|| RecipeError::new("NVIDIA native LLVM templates are unavailable"))?,
	};
	fs::read_to_string(template_path(mapping, suffix)?).map_err(|error| RecipeError::new(format!("cannot read native LLVM template: {error}")))
}

fn value_type(precision: Compute) -> &'static str {
	match precision {
		Compute::F(_) | Compute::Fp(FloatFormat { storage: FloatLayout { exp: 11, man: 52, .. }, .. }) => "double",
		Compute::Fp(FloatFormat { storage: FloatLayout { exp: 8, man: 23, .. }, .. }) | Compute::Tf(_) => "float",
		Compute::Fp(FloatFormat { storage: FloatLayout { exp: 5, man: 10, .. }, .. }) | Compute::Bf(_) => "i16",
		Compute::Fp(_) | Compute::Int(_) => "i8",
	}
}

fn pointer_type(backend: Backend) -> &'static str { if backend == Backend::Cpu { "ptr" } else { "ptr addrspace(1)" } }

fn value_literal(precision: Compute, value: f64) -> String {
	let ty = value_type(precision);
	let bits = precision.pack(value);
	match ty {
		"double" | "float" => format!("0x{bits:016X}"),
		_ => bits.to_string(),
	}
}

fn symbol_suffix(precision: Compute) -> &'static str {
	match precision_source(precision).0 {
		"default" => "",
		"-f32" => "_f32",
		"-f16" => "_f16",
		"-f8" => "_f8",
		"-bf16" => "_bf16",
		"-tf32" => "_tf32",
		"-int8" => "_int8",
		"-int4" => "_int4",
		"-int1" => "_int1",
		"-f" => "_f",
		_ => "",
	}
}

fn strip_definition(mut ir: String, name: &str) -> String {
	let needle = format!("define ");
	while let Some(relative) = ir.find(&format!("@{name}(")) {
		let start = ir[..relative].rfind(&needle).unwrap_or(relative);
		let body = &ir[relative..];
		let Some(open) = body.find('{') else { break };
		let mut depth = 0usize;
		let mut end = relative + open;
		for (index, byte) in ir[relative + open..].bytes().enumerate() {
			match byte {
				b'{' => depth += 1,
				b'}' => {
					depth = depth.saturating_sub(1);
					if depth == 0 {
						end = relative + open + index + 1;
						break;
					}
				}
				_ => {}
			}
		}
		if end <= start { break }
		ir.replace_range(start..end, "");
	}
	ir
}

fn definition(ir: &str, name: &str) -> Result<String> {
	let relative = ir.find(&format!("@{name}(")).ok_or_else(|| RecipeError::new(format!("native template definition {name} is absent")))?;
	let start = ir[..relative].rfind("define ").ok_or_else(|| RecipeError::new(format!("native template definition {name} has no header")))?;
	let body = &ir[relative..];
	let open = body.find('{').ok_or_else(|| RecipeError::new(format!("native template definition {name} has no body")))?;
	let mut depth = 0usize;
	let mut end = relative + open;
	for (offset, byte) in ir[relative + open..].bytes().enumerate() {
		match byte {
			b'{' => depth += 1,
			b'}' => {
				depth = depth.saturating_sub(1);
				if depth == 0 {
					end = relative + open + offset + 1;
					break;
				}
			}
			_ => {}
		}
	}
	require(end > start, format!("native template definition {name} is malformed"))?;
	Ok(ir[start..end].to_owned())
}

fn barrier(backend: Backend) -> &'static str {
	match backend {
		Backend::Cpu => "",
		Backend::Amd => "call void @llvm.amdgcn.s.barrier()",
		Backend::Nvidia => "call void @llvm.nvvm.barrier0()",
	}
}

fn ptr_gep(backend: Backend, base: &str, offset: usize, _ty: &str, name: &str) -> String {
	let pointer = pointer_type(backend);
	format!("%{name} = getelementptr i8, {pointer} %{base}, i32 {offset}\n")
}

impl NativeModelIr {
	pub(crate) fn emit_fixed_primitives(&self, backend: Backend, reverse: bool, training: bool) -> Result<String> {
		let mut ir = String::new();
		let order = if reverse {
			self.plans.iter().rev().enumerate().map(|(position, plan)| (self.plans.len() - position - 1, plan)).collect::<Vec<_>>()
		} else {
			self.plans.iter().enumerate().collect::<Vec<_>>()
		};
		for (index, plan) in order {
			let pointers = self.emit_pointers(backend, index, plan, reverse, &mut ir)?;
			let node = &plan.node;
			match (reverse, node.op) {
				(false, Primitive::Contraction) => {
					ir.push_str(&format!("call void @contraction_forward_body( {pointer} {source}, {pointer} {weights}, {pointer} {value}, i32 %rows, i32 {in_channels}, i32 {in_length}, i32 {out_channels}, i32 {out_length}, i32 {kernel}, i1 true, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )\n", pointer = pointer_type(backend), source = pointers.source, weights = pointers.weights, value = pointers.value, in_channels = node.input.channels, in_length = node.input.length, out_channels = node.output.channels, out_length = node.output.length, kernel = integer_argument(node.argument[0], "contraction kernel")?));
					ir.push_str(barrier(backend));
				}
				(false, Primitive::Pool) => {
					let size = integer_argument(node.argument[0], "pool size")?;
					let count = checked_mul(self.rows, node.output.elements(), "pool output count")?;
						emit_fixed_loop(&mut ir, index, "pool", count, |ir, p| {
							ir.push_str(&format!("call void @pool_forward_body( {pointer} {source}, {pointer} {value}, {pointer} {context}, i32 {p}, i32 {from}, i32 {to}, i32 {size}, i32 {channels} )\n", pointer = pointer_type(backend), source = pointers.source, value = pointers.value, context = pointers.context, p = p, from = node.input.elements(), to = node.output.elements(), size = size, channels = node.input.channels));
					})?;
					ir.push_str(barrier(backend));
				}
				(false, Primitive::Gather) => {
					let vocabulary = integer_argument(node.argument[0], "embedding vocabulary")?;
					let count = checked_mul(self.rows, node.output.elements(), "embedding output count")?;
						emit_fixed_loop(&mut ir, index, "gather", count, |ir, p| {
							ir.push_str(&format!("call void @embedding_forward_body( {pointer} {source}, {pointer} {weights}, {pointer} {value}, {pointer} {context}, i32 {p}, i32 {from}, i32 {to}, i32 {vocabulary} )\n", pointer = pointer_type(backend), source = pointers.source, weights = pointers.weights, value = pointers.value, context = pointers.context, p = p, from = node.input.elements(), to = node.output.elements(), vocabulary = vocabulary));
					})?;
					ir.push_str(barrier(backend));
				}
				(false, Primitive::Attention) => {
					if node.argument[4] == 1.0 {
						return Err(RecipeError::new("cached attention has no model-specific reverse ABI"));
					}
					ir.push_str(&format!("call void @attention_forward_body( {pointer} {source}, {pointer} {weights}, {pointer} {value}, {pointer} {context}, i32 %rows, i32 {from}, i32 {heads}, i32 {channels}, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )\n", pointer = pointer_type(backend), source = pointers.source, weights = pointers.weights, value = pointers.value, context = pointers.context, from = node.output.elements(), heads = integer_argument(node.argument[0], "attention heads")?, channels = node.output.channels));
					ir.push_str(barrier(backend));
				}
				(false, Primitive::Scan) => {
					ir.push_str(&format!("call void @scan_forward_body( {pointer} {source}, {pointer} {weights}, {pointer} {value}, {pointer} {context}, i32 %rows, i32 {in_channels}, i32 {in_length}, i32 {out_channels}, i32 {gates}, i32 %tile.m, i32 %tile.n, i32 %tile.k, i32 %threads )\n", pointer = pointer_type(backend), source = pointers.source, weights = pointers.weights, value = pointers.value, context = pointers.context, in_channels = node.input.channels, in_length = node.input.length, out_channels = node.output.channels, gates = integer_argument(node.argument[0], "scan gates")?));
						ir.push_str(barrier(backend));
				}
				(false, Primitive::Elementwise) => {
					let count = checked_mul(self.rows, node.output.elements(), "scalar output count")?;
					let pointer = pointer_type(backend);
					let ty = value_type(self.precision);
					let literal = |value: f64, ty: &str| native_literal(self.precision, ty, value);
					let prefix = format!("n{index}.scalar");
					let first = format!("%{prefix}.first");
					let second = format!("%{prefix}.second");
					let code_end = node.program_offset.checked_add(node.program_count.checked_mul(3).ok_or_else(|| RecipeError::new("scalar program length overflows"))?).ok_or_else(|| RecipeError::new("scalar program range overflows"))?;
					let code = self.graph.programs.get(node.program_offset..code_end).ok_or_else(|| RecipeError::new(format!("node {index} scalar program range is invalid")))?;
					let forward = program_ir::emit_scalar_forward(code, program_ir::ScalarContext { value_type: ty, pointer_type: pointer, first: &first, second: &second, weights: &pointers.weights, parameter_base: "0", prefix: &prefix, literal: &literal }).map_err(|error| RecipeError::new(error.to_string()))?;
					emit_fixed_loop(&mut ir, index, "scalar", count, |ir, p| {
						let first_pointer = format!("%{prefix}.first.ptr");
						let second_pointer = format!("%{prefix}.second.ptr");
						let output_pointer = format!("%{prefix}.output.ptr");
						ir.push_str(&format!("{first_pointer} = getelementptr inbounds {ty}, {pointer} {source}, i32 {p}\n{first} = load {ty}, {pointer} {first_pointer}, align {align}\n", source = pointers.source, align = alignment(ty)));
						ir.push_str(&format!("{second_pointer} = getelementptr inbounds {ty}, {pointer} {second}, i32 {p}\n", second = pointers.second));
						if pointers.second == pointers.source {
							ir.push_str(&format!("{second} = {first}\n"));
						} else {
							ir.push_str(&format!("{second} = load {ty}, {pointer} {second_pointer}, align {align}\n", align = alignment(ty)));
						}
						ir.push_str(&forward.code);
						ir.push_str(&format!("{output_pointer} = getelementptr inbounds {ty}, {pointer} {value}, i32 {p}\nstore {ty} {result}, {pointer} {output_pointer}, align {align}\n", value = pointers.value, result = forward.value, align = alignment(ty)));
					})?;
					ir.push_str(barrier(backend));
				}
				(false, Primitive::Predictor) => {
					let count = checked_mul(self.rows, node.output.elements(), "predictor output count")?;
					let locals = integer_argument(node.argument[0], "predictor locals")?;
					let pointer = pointer_type(backend);
					let ty = value_type(self.precision);
					let literal = |value: f64, ty: &str| native_literal(self.precision, ty, value);
					let prefix = format!("n{index}.predictor");
					let code_end = node.program_offset.checked_add(node.program_count.checked_mul(2).ok_or_else(|| RecipeError::new("predictor program length overflows"))?).ok_or_else(|| RecipeError::new("predictor program range overflows"))?;
					let code = self.graph.programs.get(node.program_offset..code_end).ok_or_else(|| RecipeError::new(format!("node {index} predictor program range is invalid")))?;
					let locals = usize::try_from(locals).map_err(|_| RecipeError::new("predictor locals exceed usize"))?;
					let row = format!("%{prefix}.row");
					let forward = program_ir::emit_predictor_forward(code, locals, program_ir::PredictorContext { value_type: ty, pointer_type: pointer, input: &pointers.source, row: &row, features: node.input.elements(), prefix: &prefix, literal: &literal }).map_err(|error| RecipeError::new(error.to_string()))?;
						emit_fixed_loop(&mut ir, index, "predictor", count, |ir, p| {
							ir.push_str(&format!("{row} = udiv i32 {p}, {elements}\n", elements = node.output.elements()));
							ir.push_str(&forward.code);
							let output_pointer = format!("%{prefix}.output.ptr");
							ir.push_str(&format!("{output_pointer} = getelementptr inbounds {ty}, {pointer} {value}, i32 {p}\nstore {ty} {result}, {pointer} {output_pointer}, align {align}\n", value = pointers.value, result = forward.value, align = alignment(ty)));
						})?;
					ir.push_str(barrier(backend));
				}
				(true, Primitive::Contraction) => {
						ir.push_str(&format!("call void @contraction_reverse_body( {pointer} {source}, {pointer} {weights}, {pointer} {delta}, {pointer} {source_adjoint}, {pointer} %gradient, i1 true, i1 true, i32 %rows, i32 {in_channels}, i32 {in_length}, i32 {out_channels}, i32 {out_length}, i32 {kernel}, i32 {offset}, i32 %threads )\n", pointer = pointer_type(backend), source = pointers.source, weights = pointers.weights, delta = pointers.delta, source_adjoint = pointers.source_adjoint, in_channels = node.input.channels, in_length = node.input.length, out_channels = node.output.channels, out_length = node.output.length, kernel = integer_argument(node.argument[0], "contraction kernel")?, offset = plan.node.offset));
					ir.push_str(barrier(backend));
				}
				(true, Primitive::Gather) => {
						ir.push_str(&format!("call void @embedding_reverse_body( {pointer} {source}, {pointer} {delta}, {pointer} {source_adjoint}, {pointer} %gradient, i1 {write_source}, i32 %rows, i32 {tokens}, i32 {dimensions}, i32 {vocabulary}, i32 {offset}, i32 %threads )\n", pointer = pointer_type(backend), source = pointers.source, delta = pointers.delta, source_adjoint = pointers.source_adjoint, write_source = if node.source >= 0 { "true" } else { "false" }, tokens = node.input.elements(), dimensions = node.output.channels, vocabulary = integer_argument(node.argument[0], "embedding vocabulary")?, offset = plan.node.offset));
					ir.push_str(barrier(backend));
				}
				(true, Primitive::Attention) => {
					if node.argument[4] == 1.0 {
						return Err(RecipeError::new("cached attention has no model-specific reverse ABI"));
					}
						ir.push_str(&format!("call void @attention_reverse_body( {pointer} {source}, {pointer} {weights}, {pointer} {context}, {pointer} {delta}, {pointer} {source_adjoint}, {pointer} %gradient, i1 true, i32 %rows, i32 {from}, i32 {heads}, i32 {channels}, i32 {offset}, i32 %threads )\n", pointer = pointer_type(backend), source = pointers.source, weights = pointers.weights, context = pointers.context, delta = pointers.delta, source_adjoint = pointers.source_adjoint, from = node.output.elements(), heads = integer_argument(node.argument[0], "attention heads")?, channels = node.output.channels, offset = plan.node.offset));
					ir.push_str(barrier(backend));
				}
				(true, Primitive::Scan) => {
						ir.push_str(&format!("call void @scan_reverse_body( {pointer} {source}, {pointer} {weights}, {pointer} {value}, {pointer} {context}, {pointer} {delta}, {pointer} {source_adjoint}, {pointer} %gradient, i1 true, i32 %rows, i32 {in_channels}, i32 {in_length}, i32 {out_channels}, i32 {gates}, i32 {parameters}, i32 {offset}, i32 %threads )\n", pointer = pointer_type(backend), source = pointers.source, weights = pointers.weights, value = pointers.value, context = pointers.context, delta = pointers.delta, source_adjoint = pointers.source_adjoint, in_channels = node.input.channels, in_length = node.input.length, out_channels = node.output.channels, gates = integer_argument(node.argument[0], "scan gates")?, parameters = node.parameters, offset = plan.node.offset));
						ir.push_str(barrier(backend));
				}
				(true, Primitive::Elementwise) => {
					let count = checked_mul(self.rows, node.output.elements(), "scalar reverse count")?;
					let pointer = pointer_type(backend);
					let ty = value_type(self.precision);
					let literal = |value: f64, ty: &str| native_literal(self.precision, ty, value);
					let prefix = format!("n{index}.scalar.reverse");
					let first = format!("%{prefix}.first");
					let second = format!("%{prefix}.second");
					let code_end = node.program_offset.checked_add(node.program_count.checked_mul(3).ok_or_else(|| RecipeError::new("scalar reverse program length overflows"))?).ok_or_else(|| RecipeError::new("scalar reverse program range overflows"))?;
					let code = self.graph.programs.get(node.program_offset..code_end).ok_or_else(|| RecipeError::new(format!("node {index} scalar reverse program range is invalid")))?;
					let forward = program_ir::emit_scalar_forward(code, program_ir::ScalarContext { value_type: ty, pointer_type: pointer, first: &first, second: &second, weights: &pointers.weights, parameter_base: "0", prefix: &prefix, literal: &literal }).map_err(|error| RecipeError::new(error.to_string()))?;
					let incoming = format!("%{prefix}.incoming");
					let reverse = program_ir::emit_scalar_reverse(code, &forward, program_ir::ScalarContext { value_type: ty, pointer_type: pointer, first: &first, second: &second, weights: &pointers.weights, parameter_base: "0", prefix: &prefix, literal: &literal }, &incoming).map_err(|error| RecipeError::new(error.to_string()))?;
					let gradients = reverse.parameter_adjoint.iter().map(|(&parameter, value)| Ok((checked_add(plan.node.offset, parameter, "scalar gradient offset")?, value.clone()))).collect::<Result<Vec<_>>>()?;
						emit_fixed_loop(&mut ir, index, "scalar.reverse", count, |ir, p| {
							let first_pointer = format!("%{prefix}.first.ptr");
							let second_pointer = format!("%{prefix}.second.ptr");
							let incoming_pointer = format!("%{prefix}.incoming.ptr");
							let first_adjoint_pointer = format!("%{prefix}.first.adjoint.ptr");
							ir.push_str(&format!("{first_pointer} = getelementptr inbounds {ty}, {pointer} {source}, i32 {p}\n{first} = load {ty}, {pointer} {first_pointer}, align {align}\n", source = pointers.source, align = alignment(ty)));
							ir.push_str(&format!("{second_pointer} = getelementptr inbounds {ty}, {pointer} {second}, i32 {p}\n", second = pointers.second));
							if pointers.second == pointers.source { ir.push_str(&format!("{second} = {first}\n")); } else { ir.push_str(&format!("{second} = load {ty}, {pointer} {second_pointer}, align {align}\n", align = alignment(ty))); }
							ir.push_str(&format!("{incoming_pointer} = getelementptr inbounds {ty}, {pointer} {delta}, i32 {p}\n{incoming} = load {ty}, {pointer} {incoming_pointer}, align {align}\n", delta = pointers.delta, align = alignment(ty)));
							ir.push_str(&forward.code);
							ir.push_str(&reverse.code);
							ir.push_str(&format!("{first_adjoint_pointer} = getelementptr inbounds {ty}, {pointer} {source_adjoint}, i32 {p}\n", source_adjoint = pointers.source_adjoint));
							if node.second >= 0 {
								let second_adjoint_pointer = format!("%{prefix}.second.adjoint.ptr");
								ir.push_str(&format!("call {ty} @recipe.atomic.add({pointer} {first_adjoint_pointer}, {ty} {first_adjoint})\n{second_adjoint_pointer} = getelementptr inbounds {ty}, {pointer} {second_adjoint}, i32 {p}\ncall {ty} @recipe.atomic.add({pointer} {second_adjoint_pointer}, {ty} {second_adjoint_value})\n", first_adjoint = reverse.first_adjoint, second_adjoint = pointers.second_adjoint, second_adjoint_value = reverse.second_adjoint));
							} else {
								let combined = format!("%{prefix}.combined");
								ir.push_str(&format!("{combined} = call {ty} @recipe.add({ty} {first_adjoint}, {ty} {second_adjoint})\ncall {ty} @recipe.atomic.add({pointer} {first_adjoint_pointer}, {ty} {combined})\n", first_adjoint = reverse.first_adjoint, second_adjoint = reverse.second_adjoint));
							}
							for (parameter, value) in &gradients {
								let gradient_pointer = format!("%{prefix}.gradient.{parameter}");
								ir.push_str(&format!("{gradient_pointer} = getelementptr inbounds {ty}, {pointer} %gradient, i32 {parameter}\ncall {ty} @recipe.atomic.add({pointer} {gradient_pointer}, {ty} {value})\n"));
							}
						})?;
					ir.push_str(barrier(backend));
				}
				(false, Primitive::Route) => {
					let fields = self.route_fields(backend, index, node, &mut ir)?;
					let count = checked_mul(self.rows, node.output.elements(), "route output count")?;
					let pointer = pointer_type(backend);
					let ty = value_type(self.precision);
					let prefix = format!("n{index}.route");
					let field_refs = fields.iter().map(|(source, stride, field)| program_ir::RouteField { source, stride: *stride, index: *field }).collect::<Vec<_>>();
					let row = format!("%{prefix}.row");
					let fragment = program_ir::emit_route(&field_refs, program_ir::RouteContext { value_type: ty, pointer_type: pointer, row: &row, output: &pointers.value, prefix: &prefix }).map_err(|error| RecipeError::new(error.to_string()))?;
					emit_fixed_loop(&mut ir, index, "route", count, |ir, p| {
						ir.push_str(&format!("{row} = udiv i32 {p}, {width}\n", width = field_refs.len()));
						ir.push_str(&fragment);
					})?;
					ir.push_str(barrier(backend));
				}
				(true, Primitive::Route) => {
					let fields = self.route_fields(backend, index, node, &mut ir)?;
					let count = checked_mul(self.rows, node.output.elements(), "route reverse count")?;
					let pointer = pointer_type(backend);
					let ty = value_type(self.precision);
					let prefix = format!("n{index}.route.reverse");
					let field_refs = fields.iter().map(|(source, stride, field)| program_ir::RouteField { source, stride: *stride, index: *field }).collect::<Vec<_>>();
					let mut adjoint_sources = Vec::with_capacity(node.program_count);
					for (column, value) in self.route_program(node)?.into_iter().enumerate() {
						let source = value.0;
						if source < 0 { adjoint_sources.push("%input_adjoint".to_owned()); }
						else {
							let source = usize::try_from(source).map_err(|_| RecipeError::new("route source is invalid"))?;
							let name = format!("%n{index}.route.source.{column}.adjoint");
							ir.push_str(&ptr_gep(backend, "adjoints", self.layout.adjoints[source], ty, &format!("n{index}.route.source.{column}.adjoint")));
							adjoint_sources.push(name);
						}
					}
					let adjoint_refs = adjoint_sources.iter().map(String::as_str).collect::<Vec<_>>();
					let row = format!("%{prefix}.row");
					let fragment = program_ir::emit_route_reverse(&field_refs, &adjoint_refs, program_ir::RouteReverseContext { value_type: ty, pointer_type: pointer, row: &row, output_adjoint: &pointers.delta, prefix: &prefix }).map_err(|error| RecipeError::new(error.to_string()))?;
					emit_fixed_loop(&mut ir, index, "route.reverse", count, |ir, p| {
						ir.push_str(&format!("{row} = udiv i32 {p}, {width}\n", width = field_refs.len()));
						ir.push_str(&fragment);
					})?;
					ir.push_str(barrier(backend));
				}
				(false, Primitive::Normalize) => {
					let count = checked_mul(self.rows, node.output.elements(), "normalize output count")?;
					let mode = normalize_mode(node.argument[0])?;
					let pointer = pointer_type(backend);
					let ty = value_type(self.precision);
					let prefix = format!("n{index}.normalize");
					if training && mode != program_ir::NormalizeMode::Evaluation {
						ir.push_str(&self.emit_normalize_stats(backend, index, node, &pointers, mode)?);
						ir.push_str(barrier(backend));
					}
					emit_fixed_loop(&mut ir, index, "normalize", count, |ir, p| {
						let source_pointer = format!("%{prefix}.source.ptr");
						let source_value = format!("%{prefix}.source.value");
						let row = format!("%{prefix}.row");
						ir.push_str(&format!("{source_pointer} = getelementptr inbounds {ty}, {pointer} {source}, i32 {p}\n{source_value} = load {ty}, {pointer} {source_pointer}, align {align}\n", source = pointers.source, align = alignment(ty)));
						let fragment = program_ir::emit_normalize(program_ir::NormalizeContext { value_type: ty, pointer_type: pointer, source_value: &source_value, context: &pointers.context, row: &row, rows: "%rows", channels: node.output.channels, length: node.output.length, mode, prefix: &prefix }, p);
						ir.push_str(&fragment.code);
						let output_pointer = format!("%{prefix}.output.ptr");
						ir.push_str(&format!("{output_pointer} = getelementptr inbounds {ty}, {pointer} {value}, i32 {p}\nstore {ty} {result}, {pointer} {output_pointer}, align {align}\n", value = pointers.value, result = fragment.value, align = alignment(ty)));
					})?;
					ir.push_str(barrier(backend));
				}
				(true, Primitive::Normalize) => {
					let count = checked_mul(self.rows, node.output.elements(), "normalize reverse count")?;
					let mode = normalize_mode(node.argument[0])?;
					let pointer = pointer_type(backend);
					let ty = value_type(self.precision);
					let prefix = format!("n{index}.normalize.reverse");
					emit_fixed_loop(&mut ir, index, "normalize.reverse", count, |ir, p| {
						let delta_pointer = format!("%{prefix}.delta.ptr");
						let delta_value = format!("%{prefix}.delta.value");
						let output_pointer = format!("%{prefix}.output.ptr");
						let output_value = format!("%{prefix}.output.value");
						ir.push_str(&format!("{delta_pointer} = getelementptr inbounds {ty}, {pointer} {delta}, i32 {p}\n{delta_value} = load {ty}, {pointer} {delta_pointer}, align {align}\n{output_pointer} = getelementptr inbounds {ty}, {pointer} {value}, i32 {p}\n{output_value} = load {ty}, {pointer} {output_pointer}, align {align}\n", delta = pointers.delta, value = pointers.value, align = alignment(ty)));
						let fragment = program_ir::emit_normalize_reverse(program_ir::NormalizeReverseContext { value_type: ty, pointer_type: pointer, context: &pointers.context, rows: "%rows", channels: node.output.channels, length: node.output.length, mode, prefix: &prefix }, p, &delta_value, &output_value);
						ir.push_str(&fragment.code);
						let source_pointer = format!("%{prefix}.source.adjoint.ptr");
						ir.push_str(&format!("{source_pointer} = getelementptr inbounds {ty}, {pointer} {source_adjoint}, i32 {p}\ncall {ty} @recipe.atomic.add({pointer} {source_pointer}, {ty} {contribution})\n", source_adjoint = pointers.source_adjoint, contribution = fragment.contribution));
					})?;
					ir.push_str(barrier(backend));
				}
				_ => return Err(RecipeError::new(format!("native primitive emitter does not own node {}", index))),
			}
		}
		Ok(ir)
	}

	fn emit_normalize_stats(&self, backend: Backend, index: usize, node: &Node, pointers: &ModelPointers, mode: program_ir::NormalizeMode) -> Result<String> {
		let pointer = pointer_type(backend);
		let ty = value_type(self.precision);
		let prefix = format!("n{index}.normalize.stats");
		let elements = i32::try_from(node.output.elements()).map_err(|_| RecipeError::new("normalization element count exceeds i32"))?;
		let length = i32::try_from(node.output.length).map_err(|_| RecipeError::new("normalization length exceeds i32"))?;
		let channels = i32::try_from(node.output.channels).map_err(|_| RecipeError::new("normalization channels exceed i32"))?;
		let mut ir = String::new();
		let zero = native_literal(self.precision, ty, 0.0);
		let one = native_literal(self.precision, ty, 1.0);
		let epsilon = native_literal(self.precision, ty, node.argument[1]);
		let groups = format!("%{prefix}.groups");
		let items = format!("%{prefix}.items");
		match mode {
			program_ir::NormalizeMode::Batch => {
				ir.push_str(&format!("{items} = mul i32 %rows, {length}\n", length = length));
			}
			program_ir::NormalizeMode::Layer | program_ir::NormalizeMode::Rms => {
				ir.push_str(&format!("{groups} = mul i32 %rows, {length}\n{items} = add i32 0, {channels}\n", length = length, channels = channels));
			}
			program_ir::NormalizeMode::Evaluation => return Ok(ir),
		}
		let group_limit = match mode {
			program_ir::NormalizeMode::Batch => channels.to_string(),
			program_ir::NormalizeMode::Layer | program_ir::NormalizeMode::Rms => groups.clone(),
			program_ir::NormalizeMode::Evaluation => unreachable!(),
		};
		let group = format!("%{prefix}.group");
		let mut emit_index = |code: &mut String, phase: &str, p: &str| {
			let row = format!("%{prefix}.{phase}.row");
			let position = format!("%{prefix}.{phase}.position");
			let row_base = format!("%{prefix}.{phase}.row.base");
			let channel_base = format!("%{prefix}.{phase}.channel.base");
			let local = format!("%{prefix}.{phase}.local");
			let value_index = format!("%{prefix}.{phase}.index");
			match mode {
				program_ir::NormalizeMode::Batch => {
					code.push_str(&format!("{row} = udiv i32 {p}, {length}\n{position} = urem i32 {p}, {length}\n{row_base} = mul i32 {row}, {elements}\n{channel_base} = mul i32 {group}, {length}\n{local} = add i32 {channel_base}, {position}\n{value_index} = add i32 {row_base}, {local}\n", p = p, length = length, elements = elements, group = group));
				}
				program_ir::NormalizeMode::Layer | program_ir::NormalizeMode::Rms => {
					code.push_str(&format!("{row} = udiv i32 {group}, {length}\n{position} = urem i32 {group}, {length}\n{row_base} = mul i32 {row}, {elements}\n{channel_base} = mul i32 {p}, {length}\n{local} = add i32 {channel_base}, {position}\n{value_index} = add i32 {row_base}, {local}\n", p = p, length = length, elements = elements, group = group));
				}
				program_ir::NormalizeMode::Evaluation => unreachable!(),
			}
		};
		ir.push_str(&format!("br label %{prefix}.entry\n{prefix}.entry:\nbr label %{prefix}.group.loop\n{prefix}.group.loop:\n{group} = phi i32 [ %tid, %{prefix}.entry ], [ %{prefix}.group.next, %{prefix}.store ]\n%{prefix}.group.more = icmp ult i32 {group}, {group_limit}\nbr i1 %{prefix}.group.more, label %{prefix}.mean.loop, label %{prefix}.done\n{prefix}.mean.loop:\n%{prefix}.mean.p = phi i32 [ 0, %{prefix}.group.loop ], [ %{prefix}.mean.next, %{prefix}.mean.step ]\n%{prefix}.mean.sum = phi {ty} [ {zero}, %{prefix}.group.loop ], [ %{prefix}.mean.sum.next, %{prefix}.mean.step ]\n%{prefix}.mean.more = icmp ult i32 %{prefix}.mean.p, {items}\nbr i1 %{prefix}.mean.more, label %{prefix}.mean.step, label %{prefix}.variance.loop\n{prefix}.mean.step:\n", group = group, group_limit = group_limit, ty = ty, zero = zero, items = items));
		emit_index(&mut ir, "mean", &format!("%{prefix}.mean.p"));
		ir.push_str(&format!("%{prefix}.mean.ptr = getelementptr inbounds {ty}, {pointer} {source}, i32 %{prefix}.mean.index\n%{prefix}.mean.value = load {ty}, {pointer} %{prefix}.mean.ptr, align {align}\n%{prefix}.mean.sum.next = call {ty} @recipe.add({ty} %{prefix}.mean.sum, {ty} %{prefix}.mean.value)\n%{prefix}.mean.next = add i32 %{prefix}.mean.p, 1\nbr label %{prefix}.mean.loop\n{prefix}.variance.loop:\n%{prefix}.variance.p = phi i32 [ 0, %{prefix}.mean.loop ], [ %{prefix}.variance.next, %{prefix}.variance.step ]\n%{prefix}.variance.sum = phi {ty} [ {zero}, %{prefix}.mean.loop ], [ %{prefix}.variance.sum.next, %{prefix}.variance.step ]\n%{prefix}.items.value = call {ty} @recipe.from.u32(i32 {items})\n%{prefix}.mean = call {ty} @recipe.div({ty} %{prefix}.mean.sum, {ty} %{prefix}.items.value)\n%{prefix}.variance.more = icmp ult i32 %{prefix}.variance.p, {items}\nbr i1 %{prefix}.variance.more, label %{prefix}.variance.step, label %{prefix}.store\n{prefix}.variance.step:\n", pointer = pointer, source = pointers.source, ty = ty, zero = zero, items = items, align = alignment(ty)));
		emit_index(&mut ir, "variance", &format!("%{prefix}.variance.p"));
		ir.push_str(&format!("%{prefix}.variance.ptr = getelementptr inbounds {ty}, {pointer} {source}, i32 %{prefix}.variance.index\n%{prefix}.variance.value = load {ty}, {pointer} %{prefix}.variance.ptr, align {align}\n%{prefix}.variance.centered = call {ty} @recipe.sub({ty} %{prefix}.variance.value, {ty} %{prefix}.mean)\n", pointer = pointer, source = pointers.source, ty = ty, align = alignment(ty)));
		let difference = if mode == program_ir::NormalizeMode::Rms { format!("%{prefix}.variance.value") } else { format!("%{prefix}.variance.centered") };
		ir.push_str(&format!("%{prefix}.variance.square = call {ty} @recipe.mul({ty} {difference}, {ty} {difference})\n%{prefix}.variance.sum.next = call {ty} @recipe.add({ty} %{prefix}.variance.sum, {ty} %{prefix}.variance.square)\n%{prefix}.variance.next = add i32 %{prefix}.variance.p, 1\nbr label %{prefix}.variance.loop\n{prefix}.store:\n%{prefix}.variance = call {ty} @recipe.div({ty} %{prefix}.variance.sum, {ty} %{prefix}.items.value)\n%{prefix}.adjusted = call {ty} @recipe.add({ty} %{prefix}.variance, {ty} {epsilon})\n%{prefix}.deviation = call {ty} @recipe.sqrt({ty} %{prefix}.adjusted)\n%{prefix}.scale = call {ty} @recipe.div({ty} {one}, {ty} %{prefix}.deviation)\n%{prefix}.mean.ptr = getelementptr inbounds {ty}, {pointer} {context}, i32 {group}\n%{prefix}.scale.index = add i32 {group_limit}, {group}\n%{prefix}.scale.ptr = getelementptr inbounds {ty}, {pointer} {context}, i32 %{prefix}.scale.index\n", pointer = pointer, context = pointers.context, ty = ty, epsilon = epsilon, one = one, group = group, group_limit = group_limit));
		let stored_mean = if mode == program_ir::NormalizeMode::Rms { zero.clone() } else { format!("%{prefix}.mean") };
		ir.push_str(&format!("store {ty} {stored_mean}, {pointer} %{prefix}.mean.ptr, align {align}\nstore {ty} %{prefix}.scale, {pointer} %{prefix}.scale.ptr, align {align}\n%{prefix}.group.next = add i32 {group}, %threads\nbr label %{prefix}.group.loop\n{prefix}.done:\n", pointer = pointer, ty = ty, stored_mean = stored_mean, align = alignment(ty), group = group));
		Ok(ir)
	}

	fn route_program(&self, node: &Node) -> Result<Vec<(i32, usize, usize)>> {
		let width = node.program_count.checked_mul(3).ok_or_else(|| RecipeError::new("route program length overflows"))?;
		let end = node.program_offset.checked_add(width).ok_or_else(|| RecipeError::new("route program range overflows"))?;
		let code = self.graph.programs.get(node.program_offset..end).ok_or_else(|| RecipeError::new("route program range is invalid"))?;
		code.chunks_exact(3).map(|field| {
			let source = integer_argument(field[0], "route source")?;
			let stride = usize::try_from(integer_argument(field[1], "route stride")?).map_err(|_| RecipeError::new("route stride is negative"))?;
			let index = usize::try_from(integer_argument(field[2], "route field")?).map_err(|_| RecipeError::new("route field is negative"))?;
			Ok((source, stride, index))
		}).collect()
	}

	fn route_fields(&self, backend: Backend, index: usize, node: &Node, ir: &mut String) -> Result<Vec<(String, usize, usize)>> {
		let mut fields = Vec::with_capacity(node.program_count);
		for (column, (source, stride, field)) in self.route_program(node)?.into_iter().enumerate() {
			let source_name = if source < 0 { "%samples".to_owned() } else {
				let source = usize::try_from(source).map_err(|_| RecipeError::new("route source is invalid"))?;
				let name = format!("n{index}.route.source.{column}");
				ir.push_str(&ptr_gep(backend, "values", self.layout.values[source], value_type(self.precision), &name));
				format!("%{name}")
			};
			fields.push((source_name, stride, field));
		}
		Ok(fields)
	}

	fn emit_pointers(&self, backend: Backend, index: usize, plan: &NodePlan, reverse: bool, ir: &mut String) -> Result<ModelPointers> {
		let pointer = pointer_type(backend);
		let prefix = format!("n{index}");
		let source = if plan.node.source >= 0 {
			let source = usize::try_from(plan.node.source).map_err(|_| RecipeError::new("native source node is invalid"))?;
			ptr_gep(backend, "values", self.layout.values[source], value_type(self.precision), &format!("{prefix}.source"));
			format!("%{prefix}.source")
		} else { "%samples".to_owned() };
		if plan.node.source >= 0 {
			let source = usize::try_from(plan.node.source).map_err(|_| RecipeError::new("native source node is invalid"))?;
			ir.push_str(&ptr_gep(backend, "values", self.layout.values[source], value_type(self.precision), &format!("{prefix}.source")));
		}
		let second = if plan.node.second >= 0 {
			let second = usize::try_from(plan.node.second).map_err(|_| RecipeError::new("native second node is invalid"))?;
			ir.push_str(&ptr_gep(backend, "values", self.layout.values[second], value_type(self.precision), &format!("{prefix}.second")));
			format!("%{prefix}.second")
		} else { source.clone() };
		let value = format!("%{prefix}.value");
		let context = format!("%{prefix}.context");
		let delta = format!("%{prefix}.delta");
		let weights = format!("%{prefix}.weights");
		ir.push_str(&ptr_gep(backend, "values", plan.value, value_type(self.precision), &format!("{prefix}.value")));
		ir.push_str(&ptr_gep(backend, "contexts", plan.context, value_type(self.precision), &format!("{prefix}.context")));
		ir.push_str(&ptr_gep(backend, "adjoints", plan.adjoint, value_type(self.precision), &format!("{prefix}.delta")));
		let weight_bytes = checked_mul(plan.node.offset, self.precision.bytes(), "native parameter offset")?;
		ir.push_str(&ptr_gep(backend, "weights", weight_bytes, value_type(self.precision), &format!("{prefix}.weights")));
		let source_adjoint = if plan.node.source >= 0 {
			let source = usize::try_from(plan.node.source).map_err(|_| RecipeError::new("native source adjoint node is invalid"))?;
			ir.push_str(&ptr_gep(backend, "adjoints", self.layout.adjoints[source], value_type(self.precision), &format!("{prefix}.source.adjoint")));
			format!("%{prefix}.source.adjoint")
		} else { "%input_adjoint".to_owned() };
		let second_adjoint = if reverse && plan.node.second >= 0 {
			let second = usize::try_from(plan.node.second).map_err(|_| RecipeError::new("native second adjoint node is invalid"))?;
			ir.push_str(&ptr_gep(backend, "adjoints", self.layout.adjoints[second], value_type(self.precision), &format!("{prefix}.second.adjoint")));
			format!("%{prefix}.second.adjoint")
		} else { source_adjoint.clone() };
		let _ = pointer;
		Ok(ModelPointers { source, second, value, context, delta, weights, source_adjoint, second_adjoint })
	}

	fn emit_quantized_definition(template: &str, codec: StorageCodec) -> Result<String> {
		let (label, name) = match codec {
			StorageCodec::Q4K => ("q4", "q4k"),
			StorageCodec::Q6K => ("q6", "q6k"),
			_ => return Err(RecipeError::new(format!("native quantized decoder is unavailable for {codec:?}"))),
		};
		let source = definition(template, "quantized_value")?;
		let entry = source.find("entry:").ok_or_else(|| RecipeError::new("native quantized decoder has no entry"))?;
		let branch = source.find(&format!("\n{label}:")).ok_or_else(|| RecipeError::new(format!("native quantized decoder branch {label} is absent")))? + 1;
		let end_marker = if label == "q4" { "\nq6:" } else { "\ninvalid:" };
		let end = source[branch..].find(end_marker).map(|offset| branch + offset).ok_or_else(|| RecipeError::new(format!("native quantized decoder branch {label} has no end")))?;
		let header = source[..entry].replace("@quantized_value(", &format!("@recipe_model_quantized_{name}(" )).replace(", i32 %kind", "");
		let body = source[branch..end].replacen(&format!("{label}:"), "entry:", 1);
		Ok(format!("{header}{body}\n}}\n"))
	}

	fn emit_quantized_decoders(&self, template: &str) -> Result<String> {
		let mut emitted = String::new();
		let mut seen = Vec::new();
		for plan in &self.plans {
			let Some(stored) = &plan.stored else { continue };
			let spec = stored.format.spec().ok_or_else(|| RecipeError::new(format!("native quantized format {} is unavailable", stored.format.0)))?;
			if seen.iter().any(|codec: &StorageCodec| *codec == spec.codec) { continue }
			emitted.push_str(&Self::emit_quantized_definition(template, spec.codec)?);
			seen.push(spec.codec);
		}
		Ok(emitted)
	}

	fn emit_model_load(&self, backend: Backend, template: &str) -> Result<String> {
		if self.storage_bytes == 0 { return Ok(String::new()) }
		let pointer = pointer_type(backend);
		let ty = value_type(self.precision);
		let thread = match backend {
			Backend::Cpu => "add i32 0, 0".to_owned(),
			Backend::Amd => "call i32 @llvm.amdgcn.workitem.id.x()".to_owned(),
			Backend::Nvidia => "call i32 @llvm.nvvm.read.ptx.sreg.tid.x()".to_owned(),
		};
		let kernel = match backend {
			Backend::Cpu => "",
			Backend::Amd => "protected amdgpu_kernel ",
			Backend::Nvidia => "protected ptx_kernel ",
		};
		let mut ir = format!("define {kernel}void @recipe_model_load({pointer} %weights, {pointer} %storage, {pointer} %codebook, i32 %threads) #0 {{\nentry:\n%tid = {thread}\n", kernel = kernel, pointer = pointer, thread = thread);
		let mut predecessor = "entry".to_owned();
		for (index, plan) in self.plans.iter().enumerate() {
			let Some(stored) = &plan.stored else { continue };
			let spec = stored.format.spec().ok_or_else(|| RecipeError::new(format!("native quantized format {} is unavailable", stored.format.0)))?;
			let name = match spec.codec {
				StorageCodec::Q4K => "q4k",
				StorageCodec::Q6K => "q6k",
				_ => return Err(RecipeError::new(format!("native quantized decoder is unavailable for node {index} format {:?}", spec.codec))),
			};
			let count = i32::try_from(stored.count).map_err(|_| RecipeError::new("native quantized weight count exceeds i32"))?;
			let columns = i32::try_from(stored.count.div_ceil(spec.block.max(1)) * spec.block.max(1)).map_err(|_| RecipeError::new("native quantized block count exceeds i32"))?;
			let prefix = format!("load.n{index}");
			ir.push_str(&format!("br label %{prefix}.loop\n{prefix}.loop:\n%{prefix}.p = phi i32 [ %tid, %entry ], [ %{prefix}.next, %{prefix}.step ]\n%{prefix}.more = icmp ult i32 %{prefix}.p, {count}\nbr i1 %{prefix}.more, label %{prefix}.step, label %{prefix}.done\n{prefix}.step:\n%{prefix}.storage = getelementptr i8, {pointer} %storage, i32 {storage}\n%{prefix}.weights = getelementptr {ty}, {pointer} %weights, i32 {weight}\n%{prefix}.value = call {ty} @recipe_model_quantized_{name}({pointer} %{prefix}.storage, i32 0, i32 %{prefix}.p, i32 {columns})\nstore {ty} %{prefix}.value, {pointer} %{prefix}.weights, align {align}\n%{prefix}.next = add i32 %{prefix}.p, %threads\nbr label %{prefix}.loop\n{prefix}.done:\n", pointer = pointer, ty = ty, count = count, storage = plan.storage_offset, weight = plan.node.offset, name = name, columns = columns, align = alignment(ty)).replace("%entry", &format!("%{predecessor}")));
			ir.push_str(barrier(backend));
			predecessor = format!("{prefix}.done");
		}
		ir.push_str("ret void\n}\n");
		Ok(ir)
	}

	pub(crate) fn emit(&self, backend: Backend, loss: LossFunction) -> Result<String> {
		let mut ir = backend_template(backend, self.precision)?;
		let quantized_definitions = self.emit_quantized_decoders(&ir)?;
		let model_load = self.emit_model_load(backend, &ir)?;
		for name in [
			format!("forward_graph{}", symbol_suffix(self.precision)),
			format!("tape_epoch_graph{}", symbol_suffix(self.precision)),
			"tape_forward_body".to_owned(),
			"scalar_forward_body".to_owned(),
			"scalar_reverse_body".to_owned(),
			"scalar_operand".to_owned(),
			"scalar_add_adjoint".to_owned(),
			"predictor_forward_body".to_owned(),
			"cached_attention_body".to_owned(),
			"quantized_forward_body".to_owned(),
			"quantized_value".to_owned(),
			"loss_item".to_owned(),
			"loss_gradient".to_owned(),
		] {
			ir = strip_definition(ir, &name);
		}
		ir.push_str(&quantized_definitions);
		ir.push_str(&model_load);
		let pointer = pointer_type(backend);
		let ty = value_type(self.precision);
		let align = alignment(ty);
		let kernel = match backend {
			Backend::Cpu => "",
			Backend::Amd => "protected amdgpu_kernel ",
			Backend::Nvidia => "protected ptx_kernel ",
		};
		let thread = match backend {
			Backend::Cpu => "add i32 0, 0".to_owned(),
			Backend::Amd => "call i32 @llvm.amdgcn.workitem.id.x()".to_owned(),
			Backend::Nvidia => "call i32 @llvm.nvvm.read.ptx.sreg.tid.x()".to_owned(),
		};
		let inference_forward = self.emit_fixed_primitives(backend, false, false)?;
		let training_forward = self.emit_fixed_primitives(backend, false, true)?;
		let reverse = self.emit_fixed_primitives(backend, true, false)?;
		let mut body = String::new();
		let forward_args = format!("{pointer} %samples, {pointer} %weights, {pointer} %values, {pointer} %contexts, i32 %rows, i32 %threads, i32 %tile.m, i32 %tile.n, i32 %tile.k");
		body.push_str(&format!("define internal void @recipe_model_inference_forward_body({forward_args}) #1 {{\nentry:\n%tid = {thread}\n"));
		body.push_str(&inference_forward);
		body.push_str("ret void\n}\n");
		body.push_str(&format!("define internal void @recipe_model_training_forward_body({forward_args}) #1 {{\nentry:\n%tid = {thread}\n"));
		body.push_str(&training_forward);
		body.push_str("ret void\n}\n");
		body.push_str(&format!("define {kernel}void @recipe_model_forward({forward_args}) #0 {{\nentry:\ncall void @recipe_model_inference_forward_body({forward_args})\nret void\n}}\n"));
		let gradient_bytes = checked_mul(self.graph.parameters.len(), self.precision.bytes(), "native gradient clear bytes")?;
		let input_bytes = checked_mul(checked_mul(self.rows, self.graph.input.elements(), "native input clear elements")?, self.precision.bytes(), "native input clear bytes")?;
		body.push_str(&format!("define {kernel}void @recipe_model_epoch({pointer} %samples, {pointer} %targets, {pointer} %weights, {pointer} %frozen, {pointer} %moments, {pointer} %variances, {pointer} %gradient, {pointer} %metrics, {pointer} %input_adjoint, {pointer} %values, {pointer} %contexts, {pointer} %adjoints, i32 %rows, i32 %threads, i32 %tile.m, i32 %tile.n, i32 %tile.k, {ty} %rate, {ty} %beta1, {ty} %beta2, {ty} %beta1.power, {ty} %beta2.power, {ty} %epsilon, {ty} %decay, i32 %step) #0 {{\nentry:\n%tid = {thread}\n",));
		body.push_str(&self.emit_clear_bytes(backend, "gradient", gradient_bytes, "gradient", "entry")?);
		body.push_str(&self.emit_clear_bytes(backend, "adjoints", self.layout.adjoints_bytes, "adjoints", "clear.gradient.done")?);
		body.push_str(&self.emit_clear_bytes(backend, "input_adjoint", input_bytes, "input", "clear.adjoints.done")?);
		body.push_str(barrier(backend));
		body.push_str(&format!("\ncall void @recipe_model_training_forward_body({pointer} %samples, {pointer} %weights, {pointer} %values, {pointer} %contexts, i32 %rows, i32 %threads, i32 %tile.m, i32 %tile.n, i32 %tile.k)\n"));
		body.push_str(barrier(backend));
		body.push('\n');
		body.push_str(&self.emit_loss_and_seed(backend, loss, ty, pointer, align)?);
		body.push_str(barrier(backend));
		body.push('\n');
		body.push_str(&reverse);
		body.push_str(&self.emit_adamw(backend, ty, pointer, align)?);
		body.push_str("ret void\n}\n");
		ir.push_str(&body);
		Ok(ir)
	}

	fn emit_loss_and_seed(&self, backend: Backend, loss: LossFunction, ty: &str, pointer: &str, align: usize) -> Result<String> {
		let output = self.graph.output.elements();
		let items = checked_mul(self.rows, output, "native loss items")?;
		let last = self.plans.last().ok_or_else(|| RecipeError::new("native model has no output node"))?;
		let prediction_offset = last.value;
		let adjoint_offset = last.adjoint;
		let mut ir = String::new();
		let zero = native_literal(self.precision, ty, 0.0);
		ir.push_str("br label %loss.entry\nloss.entry:\nbr label %loss.step\n");
		ir.push_str(&format!("%prediction.base = getelementptr i8, {pointer} %values, i32 {prediction_offset}\n%prediction = bitcast {pointer} %prediction.base to {pointer}\n"));
		ir.push_str(&format!("loss.step:\n%loss.p = phi i32 [ 0, %loss.entry ], [ %loss.next, %loss.item ]\n%loss.sum = phi {ty} [ {zero}, %loss.entry ], [ %loss.sum.next, %loss.item ]\n%loss.more = icmp ult i32 %loss.p, {items}\nbr i1 %loss.more, label %loss.item, label %loss.store\nloss.item:\n"));
		let prediction = format!("%loss.prediction");
		let target = format!("%loss.target");
		let pred_ptr = "%loss.prediction.ptr";
		let target_ptr = "%loss.target.ptr";
		ir.push_str(&format!("{pred_ptr} = getelementptr {ty}, {pointer} %prediction, i32 %loss.p\n{prediction} = load {ty}, {pointer} {pred_ptr}, align {align}\n{target_ptr} = getelementptr {ty}, {pointer} %targets, i32 %loss.p\n{target} = load {ty}, {pointer} {target_ptr}, align {align}\n"));
		let threshold = loss_threshold(self.precision, ty)?;
		let loss_value = emit_loss_value(&mut ir, loss, self.precision, ty, &prediction, &target, &threshold)?;
		ir.push_str(&format!("%loss.sum.next = call {ty} @recipe.add({ty} %loss.sum, {ty} {loss_value})\n%loss.next = add i32 %loss.p, 1\nbr label %loss.step\nloss.store:\n%items = call {ty} @recipe.from.u32(i32 {items})\n%loss.mean = call {ty} @recipe.div({ty} %loss.sum, {ty} %items)\n"));
		if loss.0 == 1 {
			ir.push_str(&format!("%loss.value = call {ty} @recipe.sqrt({ty} %loss.mean)\n"));
		} else {
			ir.push_str(&format!("%loss.value = call {ty} @recipe.add({ty} %loss.mean, {ty} {zero})\n"));
		}
		ir.push_str(&format!("%metric.ptr = getelementptr {ty}, {pointer} %metrics, i32 0\nstore {ty} %loss.value, {pointer} %metric.ptr, align {align}\nbr label %loss.done\nloss.done:\n"));
		ir.push_str(&format!("%adjoint.base = getelementptr i8, {pointer} %adjoints, i32 {adjoint_offset}\n%adjoint = bitcast {pointer} %adjoint.base to {pointer}\nbr label %seed.loop\nseed.loop:\n%seed.p = phi i32 [ %tid, %loss.done ], [ %seed.next, %seed.step ]\n%seed.more = icmp ult i32 %seed.p, {items}\nbr i1 %seed.more, label %seed.step, label %seed.done\nseed.step:\n%seed.pred.ptr = getelementptr {ty}, {pointer} %prediction, i32 %seed.p\n%seed.pred = load {ty}, {pointer} %seed.pred.ptr, align {align}\n%seed.target.ptr = getelementptr {ty}, {pointer} %targets, i32 %seed.p\n%seed.target = load {ty}, {pointer} %seed.target.ptr, align {align}\n",));
		let gradient = emit_loss_gradient(&mut ir, loss, self.precision, ty, "%seed.pred", "%seed.target", &threshold, "%loss.value", &format!("{items}"))?;
		ir.push_str(&format!("%seed.ptr = getelementptr {ty}, {pointer} %adjoint, i32 %seed.p\nstore {ty} {gradient}, {pointer} %seed.ptr, align {align}\n%seed.next = add i32 %seed.p, %threads\nbr label %seed.loop\nseed.done:\n"));
		Ok(ir)
	}

	fn emit_adamw(&self, backend: Backend, ty: &str, pointer: &str, align: usize) -> Result<String> {
		let parameters = i32::try_from(self.graph.parameters.len()).map_err(|_| RecipeError::new("native parameter count exceeds i32"))?;
		let mut ir = String::new();
		ir.push_str(&format!("br label %optimizer.entry\noptimizer.entry:\n%optimizer.base = add i32 0, %tid\nbr label %optimizer.loop\noptimizer.loop:\n%optimizer.p = phi i32 [ %optimizer.base, %optimizer.entry ], [ %optimizer.next, %optimizer.advance ]\n%optimizer.more = icmp ult i32 %optimizer.p, {parameters}\nbr i1 %optimizer.more, label %optimizer.step, label %optimizer.done\noptimizer.step:\n"));
		for (name, base) in [("frozen", "%frozen"), ("gradient", "%gradient"), ("moment", "%moments"), ("variance", "%variances"), ("weight", "%weights")] {
			ir.push_str(&format!("%optimizer.{name}.ptr = getelementptr {ty}, {pointer} {base}, i32 %optimizer.p\n", name = name, ty = if name == "frozen" { "i8" } else { ty }, pointer = pointer));
		}
		ir.push_str("%optimizer.frozen.value = load i8, ");
		ir.push_str(&format!("{pointer} %optimizer.frozen.ptr, align 1\n%optimizer.is.frozen = icmp ne i8 %optimizer.frozen.value, 0\nbr i1 %optimizer.is.frozen, label %optimizer.advance, label %optimizer.update\noptimizer.update:\n", pointer = pointer));
		ir.push_str(&format!("%optimizer.gradient.value = load {ty}, {pointer} %optimizer.gradient.ptr, align {align}\n%optimizer.moment.old = load {ty}, {pointer} %optimizer.moment.ptr, align {align}\n%optimizer.variance.old = load {ty}, {pointer} %optimizer.variance.ptr, align {align}\n%optimizer.weight.value = load {ty}, {pointer} %optimizer.weight.ptr, align {align}\n%optimizer.one.beta1 = call {ty} @recipe.sub({ty} {one}, {ty} %beta1)\n%optimizer.one.beta2 = call {ty} @recipe.sub({ty} {one}, {ty} %beta2)\n%optimizer.moment.part = call {ty} @recipe.mul({ty} %beta1, {ty} %optimizer.moment.old)\n%optimizer.gradient.part = call {ty} @recipe.mul({ty} %optimizer.one.beta1, {ty} %optimizer.gradient.value)\n%optimizer.moment.new = call {ty} @recipe.add({ty} %optimizer.moment.part, {ty} %optimizer.gradient.part)\n%optimizer.gradient.square = call {ty} @recipe.mul({ty} %optimizer.gradient.value, {ty} %optimizer.gradient.value)\n%optimizer.variance.part = call {ty} @recipe.mul({ty} %beta2, {ty} %optimizer.variance.old)\n%optimizer.gradient.variance = call {ty} @recipe.mul({ty} %optimizer.one.beta2, {ty} %optimizer.gradient.square)\n%optimizer.variance.new = call {ty} @recipe.add({ty} %optimizer.variance.part, {ty} %optimizer.gradient.variance)\nstore {ty} %optimizer.moment.new, {pointer} %optimizer.moment.ptr, align {align}\nstore {ty} %optimizer.variance.new, {pointer} %optimizer.variance.ptr, align {align}\n%optimizer.m.correct = call {ty} @recipe.sub({ty} {one}, {ty} %beta1.power)\n%optimizer.v.correct = call {ty} @recipe.sub({ty} {one}, {ty} %beta2.power)\n%optimizer.m.hat = call {ty} @recipe.div({ty} %optimizer.moment.new, {ty} %optimizer.m.correct)\n%optimizer.v.hat = call {ty} @recipe.div({ty} %optimizer.variance.new, {ty} %optimizer.v.correct)\n%optimizer.root = call {ty} @recipe.sqrt({ty} %optimizer.v.hat)\n%optimizer.denominator = call {ty} @recipe.add({ty} %optimizer.root, {ty} %epsilon)\n%optimizer.direction = call {ty} @recipe.div({ty} %optimizer.m.hat, {ty} %optimizer.denominator)\n%optimizer.decay = call {ty} @recipe.mul({ty} %decay, {ty} %optimizer.weight.value)\n%optimizer.total = call {ty} @recipe.add({ty} %optimizer.direction, {ty} %optimizer.decay)\n%optimizer.update = call {ty} @recipe.mul({ty} %rate, {ty} %optimizer.total)\n%optimizer.next.weight = call {ty} @recipe.sub({ty} %optimizer.weight.value, {ty} %optimizer.update)\nstore {ty} %optimizer.next.weight, {pointer} %optimizer.weight.ptr, align {align}\nbr label %optimizer.advance\noptimizer.advance:\n%optimizer.next = add i32 %optimizer.p, %threads\nbr label %optimizer.loop\noptimizer.done:\n", one = native_literal(self.precision, ty, 1.0)));
		Ok(ir)
	}

	fn emit_clear_bytes(&self, backend: Backend, base: &str, bytes: usize, label: &str, from: &str) -> Result<String> {
		let count = i32::try_from(bytes).map_err(|_| RecipeError::new(format!("native {label} clear count exceeds i32")))?;
		let pointer = pointer_type(backend);
		let prefix = format!("clear.{label}");
		let mut ir = String::new();
		ir.push_str(&format!("br label %{prefix}.loop\n{prefix}.loop:\n%{prefix}.p = phi i32 [ %tid, %{from} ], [ %{prefix}.next, %{prefix}.step ]\n%{prefix}.more = icmp ult i32 %{prefix}.p, {count}\nbr i1 %{prefix}.more, label %{prefix}.step, label %{prefix}.done\n{prefix}.step:\n%{prefix}.ptr = getelementptr i8, {pointer} %{base}, i32 %{prefix}.p\nstore i8 0, {pointer} %{prefix}.ptr, align 1\n%{prefix}.next = add i32 %{prefix}.p, %threads\nbr label %{prefix}.loop\n{prefix}.done:\n", base = base, from = from));
		Ok(ir)
	}
}

struct ModelPointers {
	source: String,
	second: String,
	value: String,
	context: String,
	delta: String,
	weights: String,
	source_adjoint: String,
	second_adjoint: String,
}

fn type_literal(ty: &str, value: f64) -> String {
	match ty {
		"double" => format!("0x{:016X}", value.to_bits()),
		"float" => format!("0x{:08X}", (value as f32).to_bits()),
		_ if value.fract() == 0.0 => (value as i64).to_string(),
		_ => value.to_string(),
	}
}

fn native_literal(precision: Compute, ty: &str, value: f64) -> String {
	match ty {
		"double" => type_literal(ty, value),
		"float" => type_literal(ty, value),
		_ => precision.pack(value).to_string(),
	}
}

fn one_literal(precision: Compute, ty: &str) -> String { native_literal(precision, ty, 1.0) }

fn normalize_mode(value: f64) -> Result<program_ir::NormalizeMode> {
	match integer_argument(value, "normalization mode")? {
		0 => Ok(program_ir::NormalizeMode::Batch),
		1 => Ok(program_ir::NormalizeMode::Layer),
		2 => Ok(program_ir::NormalizeMode::Rms),
		3 => Ok(program_ir::NormalizeMode::Evaluation),
		_ => Err(RecipeError::new("normalization mode is unsupported")),
	}
}

fn alignment(ty: &str) -> usize {
	match ty {
		"double" => 8,
		"float" | "i32" => 4,
		"i16" => 2,
		_ => 1,
	}
}

fn loss_threshold(precision: Compute, ty: &str) -> Result<String> {
	let value = env!("RECIPE_HUBER_THRESHOLD").parse::<f64>().map_err(|error| RecipeError::new(format!("invalid Huber threshold: {error}")))?;
	Ok(native_literal(precision, ty, value))
}

fn append_binary(ir: &mut String, ty: &str, name: &str, operation: &str, left: &str, right: &str) {
	ir.push_str(&format!("%{name} = call {ty} @recipe.{operation}({ty} {left}, {ty} {right})\n"));
}

fn emit_loss_value(ir: &mut String, loss: LossFunction, precision: Compute, ty: &str, prediction: &str, target: &str, threshold: &str) -> Result<String> {
	let literal = |value: f64| native_literal(precision, ty, value);
	let one = literal(1.0);
	append_binary(ir, ty, "loss.difference", "sub", prediction, target);
	append_binary(ir, ty, "loss.square", "mul", "%loss.difference", "%loss.difference");
	match loss.0 {
		0 | 1 => Ok("%loss.square".to_owned()),
		2 => {
			ir.push_str(&format!("%loss.absolute = call {ty} @recipe.abs({ty} %loss.difference)\n%loss.small = call i1 @recipe.ole({ty} %loss.absolute, {ty} {threshold})\n", ty = ty));
			append_binary(ir, ty, "loss.half.square", "mul", "%loss.square", &literal(0.5));
			append_binary(ir, ty, "loss.half.threshold", "mul", threshold, &literal(0.5));
			append_binary(ir, ty, "loss.large.base", "sub", "%loss.absolute", "%loss.half.threshold");
			append_binary(ir, ty, "loss.large", "mul", threshold, "%loss.large.base");
			ir.push_str(&format!("%loss.huber = select i1 %loss.small, {ty} %loss.half.square, {ty} %loss.large\n", ty = ty));
			Ok("%loss.huber".to_owned())
		}
		3 => {
			ir.push_str(&format!("%loss.mae = call {ty} @recipe.abs({ty} %loss.difference)\n", ty = ty));
			Ok("%loss.mae".to_owned())
		}
		4 | 5 => {
			ir.push_str(&format!("%loss.probability.raw = call {ty} @sigmoid({ty} {prediction})\n%loss.probability.low = call i1 @recipe.olt({ty} %loss.probability.raw, {ty} {tiny})\n%loss.probability.floor = select i1 %loss.probability.low, {ty} {tiny}, {ty} %loss.probability.raw\n%loss.probability.high = call i1 @recipe.ogt({ty} %loss.probability.floor, {ty} {one_minus})\n%loss.probability = select i1 %loss.probability.high, {ty} {one_minus}, {ty} %loss.probability.floor\n%loss.log.probability = call {ty} @recipe.log({ty} %loss.probability)\n%loss.one.probability = call {ty} @recipe.sub({ty} {one}, {ty} %loss.probability)\n%loss.log.one.probability = call {ty} @recipe.log({ty} %loss.one.probability)\n%loss.first = call {ty} @recipe.mul({ty} {target}, {ty} %loss.log.probability)\n%loss.one.target = call {ty} @recipe.sub({ty} {one}, {ty} {target})\n%loss.second = call {ty} @recipe.mul({ty} %loss.one.target, {ty} %loss.log.one.probability)\n%loss.cross.sum = call {ty} @recipe.add({ty} %loss.first, {ty} %loss.second)\n%loss.cross = call {ty} @recipe.neg({ty} %loss.cross.sum)\n", ty = ty, tiny = literal(f64::EPSILON), one_minus = literal(1.0 - f64::EPSILON), target = target, one = one));
			Ok("%loss.cross".to_owned())
		}
		6 => {
			ir.push_str(&format!("%loss.probability = call {ty} @sigmoid({ty} {prediction})\n%loss.target.class = call i1 @recipe.oge({ty} {target}, {ty} {half})\n%loss.one.probability = call {ty} @recipe.sub({ty} {one}, {ty} %loss.probability)\n%loss.correct.raw = select i1 %loss.target.class, {ty} %loss.probability, {ty} %loss.one.probability\n%loss.correct.low = call i1 @recipe.olt({ty} %loss.correct.raw, {ty} {tiny})\n%loss.correct = select i1 %loss.correct.low, {ty} {tiny}, {ty} %loss.correct.raw\n%loss.incorrect = call {ty} @recipe.sub({ty} {one}, {ty} %loss.correct)\n%loss.incorrect.square = call {ty} @recipe.mul({ty} %loss.incorrect, {ty} %loss.incorrect)\n%loss.log.correct = call {ty} @recipe.log({ty} %loss.correct)\n%loss.focal.product = call {ty} @recipe.mul({ty} %loss.incorrect.square, {ty} %loss.log.correct)\n%loss.focal = call {ty} @recipe.neg({ty} %loss.focal.product)\n", ty = ty, target = target, one = one, half = literal(0.5), tiny = literal(f64::EPSILON)));
			Ok("%loss.focal".to_owned())
		}
		_ => Err(RecipeError::new(format!("native loss {} is unsupported", loss.0))),
	}
}

fn emit_loss_gradient(ir: &mut String, loss: LossFunction, precision: Compute, ty: &str, prediction: &str, target: &str, threshold: &str, loss_value: &str, rows: &str) -> Result<String> {
	let literal = |value: f64| native_literal(precision, ty, value);
	let zero = literal(0.0);
	let one = literal(1.0);
	let negative_one = literal(-1.0);
	let two = literal(2.0);
	let tiny = literal(f64::EPSILON);
	let half = literal(0.5);
	append_binary(ir, ty, "seed.difference", "sub", prediction, target);
	let rows_value = "%seed.rows";
	ir.push_str(&format!("{rows_value} = call {ty} @recipe.from.u32(i32 {rows})\n", rows_value = rows_value, ty = ty, rows = rows));
	match loss.0 {
		0 => {
			append_binary(ir, ty, "seed.twice", "add", "%seed.difference", "%seed.difference");
			append_binary(ir, ty, "seed.mse", "div", "%seed.twice", rows_value);
			Ok("%seed.mse".to_owned())
		}
		1 => {
			append_binary(ir, ty, "seed.rmse.denominator", "mul", rows_value, loss_value);
			ir.push_str(&format!("%seed.rmse.zero = call i1 @recipe.oeq({ty} {loss_value}, {ty} {zero})\n", ty = ty, loss_value = loss_value, zero = zero));
			append_binary(ir, ty, "seed.rmse.divided", "div", "%seed.difference", "%seed.rmse.denominator");
			ir.push_str(&format!("%seed.rmse = select i1 %seed.rmse.zero, {ty} {zero}, {ty} %seed.rmse.divided\n", ty = ty, zero = zero));
			Ok("%seed.rmse".to_owned())
		}
		2 => {
			ir.push_str(&format!("%seed.huber.negative.threshold = call {ty} @recipe.neg({ty} {threshold})\n%seed.huber.low = call i1 @recipe.olt({ty} %seed.difference, {ty} %seed.huber.negative.threshold)\n%seed.huber.high = call i1 @recipe.ogt({ty} %seed.difference, {ty} {threshold})\n%seed.huber.lower = select i1 %seed.huber.low, {ty} %seed.huber.negative.threshold, {ty} %seed.difference\n%seed.huber.clamped = select i1 %seed.huber.high, {ty} {threshold}, {ty} %seed.huber.lower\n", ty = ty, threshold = threshold));
			append_binary(ir, ty, "seed.huber", "div", "%seed.huber.clamped", rows_value);
			Ok("%seed.huber".to_owned())
		}
		3 => {
			ir.push_str(&format!("%seed.mae.negative = call i1 @recipe.olt({ty} %seed.difference, {ty} {zero})\n%seed.mae.positive = call i1 @recipe.ogt({ty} %seed.difference, {ty} {zero})\n%seed.mae.upper = select i1 %seed.mae.positive, {ty} {one}, {ty} {zero}\n%seed.mae.sign = select i1 %seed.mae.negative, {ty} {negative_one}, {ty} %seed.mae.upper\n", ty = ty, zero = zero, one = one, negative_one = negative_one));
			append_binary(ir, ty, "seed.mae", "div", "%seed.mae.sign", rows_value);
			Ok("%seed.mae".to_owned())
		}
		4 | 5 => {
			ir.push_str(&format!("%seed.probability = call {ty} @sigmoid({ty} {prediction})\n", ty = ty, prediction = prediction));
			append_binary(ir, ty, "seed.cross.difference", "sub", "%seed.probability", target);
			append_binary(ir, ty, "seed.cross", "div", "%seed.cross.difference", rows_value);
			Ok("%seed.cross".to_owned())
		}
		6 => {
			ir.push_str(&format!("%seed.probability = call {ty} @sigmoid({ty} {prediction})\n%seed.target.class = call i1 @recipe.oge({ty} {target}, {ty} {half})\n%seed.one.probability = call {ty} @recipe.sub({ty} {one}, {ty} %seed.probability)\n%seed.correct.raw = select i1 %seed.target.class, {ty} %seed.probability, {ty} %seed.one.probability\n%seed.correct.low = call i1 @recipe.olt({ty} %seed.correct.raw, {ty} {tiny})\n%seed.correct = select i1 %seed.correct.low, {ty} {tiny}, {ty} %seed.correct.raw\n%seed.incorrect = call {ty} @recipe.sub({ty} {one}, {ty} %seed.correct)\n%seed.log.correct = call {ty} @recipe.log({ty} %seed.correct)\n", ty = ty, prediction = prediction, target = target, half = half, one = one, tiny = tiny));
			append_binary(ir, ty, "seed.focal.first", "mul", &two, "%seed.incorrect");
			append_binary(ir, ty, "seed.focal.first.value", "mul", "%seed.focal.first", "%seed.log.correct");
			append_binary(ir, ty, "seed.focal.square", "mul", "%seed.incorrect", "%seed.incorrect");
			append_binary(ir, ty, "seed.focal.second", "div", "%seed.focal.square", "%seed.correct");
			append_binary(ir, ty, "seed.focal.by.correct", "sub", "%seed.focal.first.value", "%seed.focal.second");
			append_binary(ir, ty, "seed.focal.sigmoid.derivative", "mul", "%seed.probability", "%seed.one.probability");
			ir.push_str(&format!("%seed.focal.negative.direction = call {ty} @recipe.neg({ty} %seed.focal.sigmoid.derivative)\n%seed.focal.direction = select i1 %seed.target.class, {ty} %seed.focal.sigmoid.derivative, {ty} %seed.focal.negative.direction\n", ty = ty));
			append_binary(ir, ty, "seed.focal.chain", "mul", "%seed.focal.by.correct", "%seed.focal.direction");
			append_binary(ir, ty, "seed.focal", "div", "%seed.focal.chain", rows_value);
			Ok("%seed.focal".to_owned())
		}
		_ => Err(RecipeError::new(format!("native loss {} is unsupported", loss.0))),
	}
}

fn integer_argument(value: f64, role: &str) -> Result<i32> {
	require(value.is_finite() && value.fract() == 0.0 && value >= f64::from(i32::MIN) && value <= f64::from(i32::MAX), format!("native {role} is not an integer"))?;
	Ok(value as i32)
}

fn emit_fixed_loop(ir: &mut String, index: usize, name: &str, count: usize, mut body: impl FnMut(&mut String, &str)) -> Result<()> {
	let prefix = format!("n{index}.{name}");
	let count = i32::try_from(count).map_err(|_| RecipeError::new(format!("native {name} loop count exceeds i32")))?;
	ir.push_str(&format!("br label %{prefix}.entry\n{prefix}.entry:\nbr label %{prefix}.loop\n{prefix}.loop:\n%{prefix}.p = phi i32 [ %tid, %{prefix}.entry ], [ %{prefix}.next, %{prefix}.step ]\n%{prefix}.more = icmp ult i32 %{prefix}.p, {count}\nbr i1 %{prefix}.more, label %{prefix}.body, label %{prefix}.done\n{prefix}.body:\n"));
	body(ir, &format!("%{prefix}.p"));
	ir.push_str(&format!("br label %{prefix}.step\n{prefix}.step:\n%{prefix}.next = add i32 %{prefix}.p, %threads\nbr label %{prefix}.loop\n{prefix}.done:\n"));
	Ok(())
}
