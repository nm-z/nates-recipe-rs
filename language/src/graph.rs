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

	fn topological_order_from(&self, producers: &BTreeMap<ValueId, KernelTemplateId>) -> LanguageResult<Vec<KernelTemplateId>> {
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
