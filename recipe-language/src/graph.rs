use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{KernelTemplateId, ValueId};

use crate::{LanguageError, LanguageErrorKind, LanguageResult, PrimitiveKernel, Tensor};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalculationNode {
	pub kernel: PrimitiveKernel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalculationGraph {
	pub tensors: Vec<Tensor>,
	pub nodes: Vec<CalculationNode>,
}

impl CalculationGraph {
	/// Assemble independently materialized operation fragments into one
	/// canonical model graph.
	///
	/// Fragment-local external flags are intentionally discarded: they describe
	/// each fragment's temporary boundary, not the complete run boundary. The
	/// caller supplies the exact external input and output value sets for the
	/// assembled graph. Repeated tensor declarations may differ only in those
	/// two boundary flags; dtype, shape, layout, and storage must match exactly.
	pub fn assemble(
		fragments: impl IntoIterator<Item = Self>,
		external_inputs: impl IntoIterator<Item = ValueId>,
		external_outputs: impl IntoIterator<Item = ValueId>,
	) -> LanguageResult<Self> {
		let external_inputs = unique_boundary("external input", external_inputs)?;
		let external_outputs = unique_boundary("external output", external_outputs)?;
		let mut tensors = BTreeMap::<ValueId, Tensor>::new();
		let mut nodes = Vec::new();
		for fragment in fragments {
			for tensor in fragment.tensors {
				match tensors.get(&tensor.id) {
					Some(previous) if same_storage_contract(previous, &tensor) => {}
					Some(_) => {
						return Err(LanguageError::new(
							LanguageErrorKind::DuplicateTensor,
							"fragments declare different storage contracts for the same tensor",
						)
						.for_value(tensor.id));
					}
					None => {
						tensors.insert(tensor.id, tensor);
					}
				}
			}
			nodes.extend(fragment.nodes);
		}
		for value in external_inputs.union(&external_outputs) {
			if !tensors.contains_key(value) {
				return Err(LanguageError::new(
					LanguageErrorKind::UnknownTensor,
					"assembled graph boundary references an absent tensor",
				)
				.for_value(*value));
			}
		}
		let mut tensors = tensors
			.into_iter()
			.map(|(id, mut tensor)| {
				tensor.external_input = external_inputs.contains(&id);
				tensor.external_output = external_outputs.contains(&id);
				tensor
			})
			.collect::<Vec<_>>();
		tensors.sort_by_key(|tensor| tensor.id);
		nodes.sort_by_key(|node| node.kernel.id);
		let graph = Self { tensors, nodes };
		graph.validate()?;
		Ok(graph)
	}

	pub fn validate(&self) -> LanguageResult<()> {
		let tensors = self.tensor_index()?;
		for tensor in tensors.values() {
			tensor.validate()?;
		}

		let mut kernels = BTreeSet::new();
		let mut producers = BTreeMap::<ValueId, KernelTemplateId>::new();
		for node in &self.nodes {
			if !kernels.insert(node.kernel.id) {
				return Err(LanguageError::new(
					LanguageErrorKind::DuplicateKernel,
					format!("kernel {} appears more than once", node.kernel.id),
				)
				.for_kernel(node.kernel.id));
			}
			node.kernel.validate(&tensors)?;
			for output in &node.kernel.outputs {
				if let Some(previous) = producers.insert(*output, node.kernel.id) {
					return Err(LanguageError::new(
						LanguageErrorKind::DuplicateProducer,
						format!(
							"tensor {output} is produced by kernels {previous} and {}",
							node.kernel.id
						),
					)
					.for_kernel(node.kernel.id)
					.for_value(*output));
				}
			}
		}

		for tensor in tensors.values() {
			match (tensor.external_input, producers.get(&tensor.id)) {
				(true, Some(producer)) => {
					return Err(LanguageError::new(
						LanguageErrorKind::DuplicateProducer,
						format!(
							"external input tensor {} is also produced by kernel {producer}",
							tensor.id
						),
					)
					.for_value(tensor.id));
				}
				(false, None) => {
					return Err(LanguageError::new(
						LanguageErrorKind::MissingProducer,
						format!(
							"non-external tensor {} has no calculation producer",
							tensor.id
						),
					)
					.for_value(tensor.id));
				}
				_ => {}
			}
		}

		self.topological_order_from(&producers)?;
		Ok(())
	}

	pub fn topological_order(&self) -> LanguageResult<Vec<KernelTemplateId>> {
		self.validate()?;
		let producers = self
			.nodes
			.iter()
			.flat_map(|node| {
				node.kernel
					.outputs
					.iter()
					.map(move |value| (*value, node.kernel.id))
			})
			.collect::<BTreeMap<_, _>>();
		self.topological_order_from(&producers)
	}

	pub fn dependencies(&self, kernel: KernelTemplateId) -> LanguageResult<Vec<KernelTemplateId>> {
		self.validate()?;
		let producers = self
			.nodes
			.iter()
			.flat_map(|node| {
				node.kernel
					.outputs
					.iter()
					.map(move |value| (*value, node.kernel.id))
			})
			.collect::<BTreeMap<_, _>>();
		let node = self
			.nodes
			.iter()
			.find(|node| node.kernel.id == kernel)
			.ok_or_else(|| {
				LanguageError::new(
					LanguageErrorKind::InvalidPrimitive,
					format!("kernel {kernel} is absent"),
				)
				.for_kernel(kernel)
			})?;
		let mut dependencies = node
			.kernel
			.inputs
			.iter()
			.filter_map(|value| producers.get(value).copied())
			.collect::<Vec<_>>();
		dependencies.sort_unstable();
		dependencies.dedup();
		Ok(dependencies)
	}

	fn tensor_index(&self) -> LanguageResult<BTreeMap<ValueId, &Tensor>> {
		let mut tensors = BTreeMap::new();
		for tensor in &self.tensors {
			if tensors.insert(tensor.id, tensor).is_some() {
				return Err(LanguageError::new(
					LanguageErrorKind::DuplicateTensor,
					format!("tensor {} appears more than once", tensor.id),
				)
				.for_value(tensor.id));
			}
		}
		Ok(tensors)
	}

	fn topological_order_from(
		&self,
		producers: &BTreeMap<ValueId, KernelTemplateId>,
	) -> LanguageResult<Vec<KernelTemplateId>> {
		let mut indegrees = self
			.nodes
			.iter()
			.map(|node| (node.kernel.id, 0_usize))
			.collect::<BTreeMap<_, _>>();
		let mut successors = BTreeMap::<KernelTemplateId, Vec<KernelTemplateId>>::new();
		let mut edges = BTreeSet::new();
		for node in &self.nodes {
			for dependency in node
				.kernel
				.inputs
				.iter()
				.filter_map(|input| producers.get(input).copied())
			{
				if dependency == node.kernel.id {
					return Err(LanguageError::new(
						LanguageErrorKind::Cycle,
						"kernel consumes its own output value",
					)
					.for_kernel(node.kernel.id));
				}
				if edges.insert((dependency, node.kernel.id)) {
					*indegrees
						.get_mut(&node.kernel.id)
						.expect("kernel is indexed") += 1;
					successors
						.entry(dependency)
						.or_default()
						.push(node.kernel.id);
				}
			}
		}
		for values in successors.values_mut() {
			values.sort_unstable();
		}

		let mut ready = indegrees
			.iter()
			.filter_map(|(id, degree)| (*degree == 0).then_some(*id))
			.collect::<BTreeSet<_>>();
		let mut order = Vec::with_capacity(self.nodes.len());
		while let Some(id) = ready.pop_first() {
			order.push(id);
			for successor in successors.get(&id).into_iter().flatten() {
				let degree = indegrees.get_mut(successor).expect("successor is indexed");
				*degree -= 1;
				if *degree == 0 {
					ready.insert(*successor);
				}
			}
		}
		if order.len() != self.nodes.len() {
			return Err(LanguageError::new(
				LanguageErrorKind::Cycle,
				"calculation graph contains a cycle",
			));
		}
		Ok(order)
	}
}

fn unique_boundary(name: &str, values: impl IntoIterator<Item = ValueId>) -> LanguageResult<BTreeSet<ValueId>> {
	let mut result = BTreeSet::new();
	for value in values {
		if !result.insert(value) {
			return Err(LanguageError::new(
				LanguageErrorKind::DuplicateTensor,
				format!("{name} boundary repeats tensor {value}"),
			)
			.for_value(value));
		}
	}
	Ok(result)
}

fn same_storage_contract(left: &Tensor, right: &Tensor) -> bool {
	left.id == right.id
		&& left.dtype == right.dtype
		&& left.shape == right.shape
		&& left.layout == right.layout
		&& left.storage_bytes == right.storage_bytes
}

#[cfg(test)]
mod tests {
	use recipe_core::{
		AliasPermission, DType, KernelTemplateId, ScalarInput, ScalarInstruction, ScalarOpcode, ScalarProgram,
		ScalarValueId, ValueId,
	};

	use super::*;
	use crate::{Elementwise, PrimitiveAliasRule, PrimitiveKind, Shape};

	fn tensor(id: u64, external: bool) -> Tensor {
		Tensor::contiguous(
			ValueId::new(id),
			DType::F32,
			Shape::new(vec![16]).unwrap(),
			external,
			false,
		)
		.unwrap()
	}

	fn add(id: u64, left: u64, right: u64, output: u64) -> CalculationNode {
		let lhs = ScalarValueId::new(1);
		let rhs = ScalarValueId::new(2);
		let result = ScalarValueId::new(3);
		CalculationNode {
			kernel: PrimitiveKernel {
				id: KernelTemplateId::new(id),
				inputs: vec![ValueId::new(left), ValueId::new(right)],
				outputs: vec![ValueId::new(output)],
				alias_rules: vec![
					PrimitiveAliasRule {
						input: 0,
						output: 0,
						permission: AliasPermission::MayAliasExact,
					},
					PrimitiveAliasRule {
						input: 1,
						output: 0,
						permission: AliasPermission::Forbidden,
					},
				],
				kind: PrimitiveKind::Elementwise(Elementwise {
					program: ScalarProgram {
						inputs: vec![
							ScalarInput {
								id: lhs,
								dtype: DType::F32,
							},
							ScalarInput {
								id: rhs,
								dtype: DType::F32,
							},
						],
						constants: Vec::new(),
						instructions: vec![ScalarInstruction {
							result,
							dtype: DType::F32,
							opcode: ScalarOpcode::Add,
							operands: vec![lhs, rhs],
						}],
						outputs: vec![result],
					},
				}),
			},
		}
	}

	#[test]
	fn derives_deterministic_data_dependencies() {
		let graph = CalculationGraph {
			tensors: vec![
				tensor(1, true),
				tensor(2, true),
				tensor(3, false),
				tensor(4, false),
			],
			nodes: vec![add(2, 3, 1, 4), add(1, 1, 2, 3)],
		};
		assert_eq!(
			graph.topological_order().unwrap(),
			[KernelTemplateId::new(1), KernelTemplateId::new(2)]
		);
		assert_eq!(
			graph.dependencies(KernelTemplateId::new(2)).unwrap(),
			[KernelTemplateId::new(1)]
		);
	}

	#[test]
	fn rejects_a_tensor_with_two_producers() {
		let graph = CalculationGraph {
			tensors: vec![tensor(1, true), tensor(2, true), tensor(3, false)],
			nodes: vec![add(1, 1, 2, 3), add(2, 2, 1, 3)],
		};
		assert_eq!(
			graph.validate().unwrap_err().kind,
			LanguageErrorKind::DuplicateProducer
		);
	}

	#[test]
	fn assembles_fragment_boundaries_into_one_internal_dependency() {
		let mut first_output = tensor(3, false);
		first_output.external_output = true;
		let first = CalculationGraph {
			tensors: vec![tensor(1, true), tensor(2, true), first_output],
			nodes: vec![add(1, 1, 2, 3)],
		};
		first.validate().unwrap();

		let second = CalculationGraph {
			tensors: vec![tensor(3, true), tensor(4, true), tensor(5, false)],
			nodes: vec![add(2, 3, 4, 5)],
		};
		second.validate().unwrap();

		let graph = CalculationGraph::assemble(
			[first, second],
			[ValueId::new(1), ValueId::new(2), ValueId::new(4)],
			[ValueId::new(5)],
		)
		.unwrap();
		assert_eq!(
			graph.topological_order().unwrap(),
			[KernelTemplateId::new(1), KernelTemplateId::new(2)]
		);
		let internal = graph
			.tensors
			.iter()
			.find(|tensor| tensor.id == ValueId::new(3))
			.unwrap();
		assert!(!internal.external_input);
		assert!(!internal.external_output);
	}

	#[test]
	fn fragment_tensor_storage_contracts_must_match_exactly() {
		let left = CalculationGraph {
			tensors: vec![tensor(1, true)],
			nodes: Vec::new(),
		};
		let mut conflicting = tensor(1, true);
		conflicting.dtype = DType::I32;
		let right = CalculationGraph {
			tensors: vec![conflicting],
			nodes: Vec::new(),
		};
		let error = CalculationGraph::assemble([left, right], [ValueId::new(1)], []).unwrap_err();
		assert_eq!(error.kind, LanguageErrorKind::DuplicateTensor);
		assert_eq!(error.value, Some(ValueId::new(1)));
	}
}
