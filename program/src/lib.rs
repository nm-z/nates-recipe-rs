#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Static lifecycle envelope for Recipe calculation graphs.
//!
//! [`CalculationGraph`](recipe_language::CalculationGraph) remains an acyclic
//! description of calculation dependencies. This crate assigns those kernels
//! to explicit subsets of a finite or gracefully stopped unbounded loop without unrolling the graph or its
//! artifacts. Init admission and exit remain separate runtime phases.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use recipe_core::{ByteCount, KernelTemplateId, MetricId, ValueId};
pub use recipe_core::{IterationDomain, LoopIterations};
use recipe_language::{CalculationGraph, OgdlCodecError};
use recipe_ogdl::{Graph, GraphError, NodeId, ParseError};

const PROGRAM_ROOT: &str = "RecipeProgram";
const PROGRAM_SCHEMA: &str = "StaticCalculationProgram";
const PROGRAM_VERSION: &str = "2";
const LEGACY_PROGRAM_VERSION: &str = "1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KernelIterationDomain {
	pub kernel: KernelTemplateId,
	pub domain: IterationDomain,
}

/// One user-facing scalar emitted through a preallocated, nonblocking metric
/// slot on an explicit subset of the finite or unbounded loop.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetricEmission {
	pub metric: MetricId,
	pub value: ValueId,
	pub domain: IterationDomain,
}

/// One acyclic calculation graph plus an explicit bounded activation domain
/// for every source kernel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StaticCalculationProgram {
	graph: CalculationGraph,
	iterations: LoopIterations,
	domains: Vec<KernelIterationDomain>,
	metrics: Vec<MetricEmission>,
}

impl StaticCalculationProgram {
	pub fn new<I: Into<LoopIterations>>(
		graph: CalculationGraph,
		iterations: I,
		domains: Vec<KernelIterationDomain>,
	) -> ProgramResult<Self> {
		Self::new_with_metrics(graph, iterations, domains, Vec::new())
	}

	pub fn new_with_metrics<I: Into<LoopIterations>>(
		graph: CalculationGraph,
		iterations: I,
		mut domains: Vec<KernelIterationDomain>,
		mut metrics: Vec<MetricEmission>,
	) -> ProgramResult<Self> {
		let iterations = iterations.into();
		domains.sort_by_key(|entry| entry.kernel);
		metrics.sort_by_key(|entry| entry.metric);
		let program = Self {
			graph,
			iterations,
			domains,
			metrics,
		};
		program.validate()?;
		Ok(program)
	}

	pub fn every_iteration<I: Into<LoopIterations>>(graph: CalculationGraph, iterations: I) -> ProgramResult<Self> {
		let iterations = iterations.into();
		let domains = graph
			.nodes
			.iter()
			.map(|node| {
				KernelIterationDomain {
					kernel: node.kernel.id,
					domain: IterationDomain::every(iterations),
				}
			})
			.collect();
		Self::new(graph, iterations, domains)
	}

	pub fn with_metrics(mut self, mut metrics: Vec<MetricEmission>) -> ProgramResult<Self> {
		metrics.sort_by_key(|entry| entry.metric);
		self.metrics = metrics;
		self.validate()?;
		Ok(self)
	}

	pub fn validate(&self) -> ProgramResult<()> {
		self.graph
			.validate()
			.map_err(|error| ProgramError::Graph(OgdlCodecError::InvalidGraph(error)))?;
		let kernels = self
			.graph
			.nodes
			.iter()
			.map(|node| node.kernel.id)
			.collect::<BTreeSet<_>>();
		let mut assigned = BTreeSet::new();
		let mut domain_by_kernel = BTreeMap::new();
		for assignment in &self.domains {
			if !kernels.contains(&assignment.kernel) {
				return Err(ProgramError::new(
					ProgramErrorKind::UnknownKernel,
					format!(
						"iteration domain references unknown kernel {}",
						assignment.kernel
					),
				));
			}
			if !assigned.insert(assignment.kernel) {
				return Err(ProgramError::new(
					ProgramErrorKind::DuplicateKernel,
					format!(
						"kernel {} has more than one iteration domain",
						assignment.kernel
					),
				));
			}
			domain_by_kernel.insert(assignment.kernel, assignment.domain);
			if !assignment.domain.is_within(self.iterations) {
				return Err(ProgramError::new(
					ProgramErrorKind::InvalidIterationDomain,
					format!(
						"iteration domain [{}, {}) exceeds loop [0, {})",
						assignment.domain.first_iteration(),
						domain_end_text(assignment.domain),
						loop_end_text(self.iterations),
					),
				));
			}
		}
		if assigned != kernels {
			let missing = kernels
				.difference(&assigned)
				.next()
				.copied()
				.expect("different finite sets have one differing element");
			return Err(ProgramError::new(
				ProgramErrorKind::MissingKernel,
				format!("kernel {missing} has no explicit iteration domain"),
			));
		}
		let producers = self
			.graph
			.nodes
			.iter()
			.flat_map(|node| {
				node.kernel
					.outputs
					.iter()
					.map(move |output| (*output, node.kernel.id))
			})
			.collect::<BTreeMap<_, _>>();
		for consumer in &self.graph.nodes {
			let consumer_domain = domain_by_kernel[&consumer.kernel.id];
			for input in &consumer.kernel.inputs {
				let Some(producer) = producers.get(input).copied() else {
					continue;
				};
				let producer_domain = domain_by_kernel[&producer];
				if producer_domain.first_iteration() > consumer_domain.first_iteration() {
					return Err(ProgramError::new(
						ProgramErrorKind::UninitializedDependency,
						format!(
							"kernel {} first consumes value {input} at iteration {}, before producer {producer} first runs at iteration {}",
							consumer.kernel.id,
							consumer_domain.first_iteration(),
							producer_domain.first_iteration()
						),
					));
				}
			}
		}
		let tensors = self
			.graph
			.tensors
			.iter()
			.map(|tensor| (tensor.id, tensor))
			.collect::<BTreeMap<_, _>>();
		let mut metric_ids = BTreeSet::new();
		for emission in &self.metrics {
			if emission.metric.get() == 0 {
				return Err(ProgramError::new(
					ProgramErrorKind::InvalidMetricId,
					"metric ID zero is reserved",
				));
			}
			if !metric_ids.insert(emission.metric) {
				return Err(ProgramError::new(
					ProgramErrorKind::DuplicateMetric,
					format!("metric {} is declared more than once", emission.metric),
				));
			}
			if emission.value.get() == 0 {
				return Err(ProgramError::new(
					ProgramErrorKind::InvalidMetricValue,
					format!("metric {} uses reserved value ID zero", emission.metric),
				));
			}
			let tensor = tensors.get(&emission.value).copied().ok_or_else(|| {
				ProgramError::new(
					ProgramErrorKind::UnknownMetricValue,
					format!(
						"metric {} references unknown value {}",
						emission.metric, emission.value
					),
				)
			})?;
			if tensor.shape.elements() != 1 || tensor.storage_bytes != ByteCount::new(4) {
				return Err(ProgramError::new(
					ProgramErrorKind::InvalidMetricValue,
					format!(
						"metric {} value {} must be exactly one {:?} scalar stored in four bytes",
						emission.metric, emission.value, tensor.dtype
					),
				));
			}
			if tensor.external_output {
				return Err(ProgramError::new(
					ProgramErrorKind::InvalidMetricValue,
					format!(
						"metric {} value {} must remain loop-internal mailbox telemetry, not an external output",
						emission.metric, emission.value
					),
				));
			}
			if !emission.domain.is_within(self.iterations) {
				return Err(ProgramError::new(
					ProgramErrorKind::InvalidIterationDomain,
					format!(
						"metric {} domain [{}, {}) stride {} exceeds loop [0, {})",
						emission.metric,
						emission.domain.first_iteration(),
						domain_end_text(emission.domain),
						emission.domain.stride(),
						loop_end_text(self.iterations),
					),
				));
			}
			let producer = producers.get(&emission.value).copied().ok_or_else(|| {
				ProgramError::new(
					ProgramErrorKind::UnproducedMetricValue,
					format!(
						"metric {} value {} has no calculation producer",
						emission.metric, emission.value
					),
				)
			})?;
			let producer_domain = domain_by_kernel[&producer];
			if !domain_covers(producer_domain, emission.domain) {
				return Err(ProgramError::new(
					ProgramErrorKind::UncoveredMetricDomain,
					format!(
						"metric {} domain [{}, {}) stride {} is not covered by producer {producer} domain [{}, {}) stride {}",
						emission.metric,
						emission.domain.first_iteration(),
						domain_end_text(emission.domain),
						emission.domain.stride(),
						producer_domain.first_iteration(),
						domain_end_text(producer_domain),
						producer_domain.stride()
					),
				));
			}
		}
		Ok(())
	}

	#[must_use]
	pub const fn graph(&self) -> &CalculationGraph { &self.graph }

	#[must_use]
	pub const fn iterations(&self) -> LoopIterations { self.iterations }

	#[must_use]
	pub fn domains(&self) -> &[KernelIterationDomain] { &self.domains }

	#[must_use]
	pub fn domain(&self, kernel: KernelTemplateId) -> Option<IterationDomain> {
		self.domains
			.binary_search_by_key(&kernel, |entry| entry.kernel)
			.ok()
			.map(|index| self.domains[index].domain)
	}

	#[must_use]
	pub fn metrics(&self) -> &[MetricEmission] { &self.metrics }

	#[must_use]
	pub fn metric(&self, metric: MetricId) -> Option<MetricEmission> {
		self.metrics
			.binary_search_by_key(&metric, |entry| entry.metric)
			.ok()
			.map(|index| self.metrics[index])
	}

	pub fn to_ogdl(&self) -> ProgramResult<String> {
		self.validate()?;
		let graph = self.to_ogdl_graph()?;
		Ok(graph.to_canonical_string())
	}

	pub fn to_ogdl_graph(&self) -> ProgramResult<Graph> {
		self.validate()?;
		let mut output = Graph::new();
		let program = output.append_root(PROGRAM_ROOT)?;
		append_field(&mut output, program, "schema", PROGRAM_SCHEMA)?;
		append_field(&mut output, program, "version", PROGRAM_VERSION)?;
		append_field(
			&mut output,
			program,
			"iterations",
			loop_end_text(self.iterations),
		)?;
		let domains = output.append_child(program, "domains")?;
		for assignment in &self.domains {
			let domain = output.append_child(domains, "domain")?;
			append_field(
				&mut output,
				domain,
				"kernel",
				assignment.kernel.get().to_string(),
			)?;
			append_field(
				&mut output,
				domain,
				"first",
				assignment.domain.first_iteration().to_string(),
			)?;
			append_field(
				&mut output,
				domain,
				"end_exclusive",
				domain_end_text(assignment.domain),
			)?;
			append_field(
				&mut output,
				domain,
				"stride",
				assignment.domain.stride().get().to_string(),
			)?;
		}
		let metrics = output.append_child(program, "metrics")?;
		for emission in &self.metrics {
			let metric = output.append_child(metrics, "metric")?;
			append_field(&mut output, metric, "id", emission.metric.get().to_string())?;
			append_field(
				&mut output,
				metric,
				"value",
				emission.value.get().to_string(),
			)?;
			append_field(
				&mut output,
				metric,
				"first",
				emission.domain.first_iteration().to_string(),
			)?;
			append_field(
				&mut output,
				metric,
				"end_exclusive",
				domain_end_text(emission.domain),
			)?;
			append_field(
				&mut output,
				metric,
				"stride",
				emission.domain.stride().get().to_string(),
			)?;
		}
		let calculation = self.graph.to_ogdl_graph()?;
		let calculation_root = *calculation.roots().first().ok_or_else(|| {
			ProgramError::new(
				ProgramErrorKind::InvalidDocument,
				"calculation graph OGDL has no root",
			)
		})?;
		copy_subtree(&calculation, calculation_root, &mut output, None)?;
		Ok(output)
	}

	pub fn from_ogdl(input: &str) -> ProgramResult<Self> {
		let graph = Graph::parse(input)?;
		Self::from_ogdl_graph(&graph)
	}

	pub fn from_ogdl_graph(source: &Graph) -> ProgramResult<Self> {
		if source.roots().len() != 2 {
			return Err(ProgramError::new(
				ProgramErrorKind::InvalidDocument,
				format!(
					"static program requires RecipeProgram and RecipeIR roots, got {}",
					source.roots().len()
				),
			));
		}
		let program_root = source.roots()[0];
		let calculation_root = source.roots()[1];
		require_text(source, program_root, PROGRAM_ROOT, "root")?;
		require_text(source, calculation_root, "RecipeIR", "calculation root")?;
		let version_field = unique_field(source, program_root, "version", "RecipeProgram")?;
		let version = leaf_value(source, version_field, "RecipeProgram.version")?;
		let fields = match version {
			LEGACY_PROGRAM_VERSION => {
				exact_fields(
					source,
					program_root,
					&["schema", "version", "iterations", "domains"],
					"RecipeProgram",
				)?
			}
			PROGRAM_VERSION => {
				exact_fields(
					source,
					program_root,
					&["schema", "version", "iterations", "domains", "metrics"],
					"RecipeProgram",
				)?
			}
			_ => {
				return Err(ProgramError::new(
					ProgramErrorKind::InvalidDocument,
					format!("RecipeProgram.version is {version:?}, expected \"1\" or \"2\""),
				));
			}
		};
		require_leaf_value(
			source,
			fields["schema"],
			PROGRAM_SCHEMA,
			"RecipeProgram.schema",
		)?;
		let iterations = parse_loop_iterations(leaf_value(
			source,
			fields["iterations"],
			"RecipeProgram.iterations",
		)?)?;
		let domains_node = fields["domains"];
		let domain_children = node(source, domains_node, "RecipeProgram.domains")?.children();
		let mut domains = Vec::with_capacity(domain_children.len());
		for (index, child) in domain_children.iter().copied().enumerate() {
			let path = format!("RecipeProgram.domains[{index}]");
			require_text(source, child, "domain", &path)?;
			let fields = exact_fields(
				source,
				child,
				&["kernel", "first", "end_exclusive", "stride"],
				&path,
			)?;
			let kernel = KernelTemplateId::new(parse_u64(
				leaf_value(source, fields["kernel"], &format!("{path}.kernel"))?,
				&format!("{path}.kernel"),
			)?);
			let first = parse_u64(
				leaf_value(source, fields["first"], &format!("{path}.first"))?,
				&format!("{path}.first"),
			)?;
			let end_exclusive = leaf_value(
				source,
				fields["end_exclusive"],
				&format!("{path}.end_exclusive"),
			)?;
			let stride = parse_u64(
				leaf_value(source, fields["stride"], &format!("{path}.stride"))?,
				&format!("{path}.stride"),
			)?;
			domains.push(KernelIterationDomain {
				kernel,
				domain: parse_iteration_domain(first, end_exclusive, stride, &path)?,
			});
		}
		let mut metrics = Vec::new();
		if let Some(metrics_node) = fields.get("metrics").copied() {
			let metric_children = node(source, metrics_node, "RecipeProgram.metrics")?.children();
			metrics.reserve(metric_children.len());
			for (index, child) in metric_children.iter().copied().enumerate() {
				let path = format!("RecipeProgram.metrics[{index}]");
				require_text(source, child, "metric", &path)?;
				let fields = exact_fields(
					source,
					child,
					&["id", "value", "first", "end_exclusive", "stride"],
					&path,
				)?;
				let metric = MetricId::new(parse_u64(
					leaf_value(source, fields["id"], &format!("{path}.id"))?,
					&format!("{path}.id"),
				)?);
				let value = ValueId::new(parse_u64(
					leaf_value(source, fields["value"], &format!("{path}.value"))?,
					&format!("{path}.value"),
				)?);
				let first = parse_u64(
					leaf_value(source, fields["first"], &format!("{path}.first"))?,
					&format!("{path}.first"),
				)?;
				let end_exclusive = leaf_value(
					source,
					fields["end_exclusive"],
					&format!("{path}.end_exclusive"),
				)?;
				let stride = parse_u64(
					leaf_value(source, fields["stride"], &format!("{path}.stride"))?,
					&format!("{path}.stride"),
				)?;
				metrics.push(MetricEmission {
					metric,
					value,
					domain: parse_iteration_domain(first, end_exclusive, stride, &path)?,
				});
			}
		}
		let mut calculation = Graph::new();
		copy_subtree(source, calculation_root, &mut calculation, None)?;
		let graph = CalculationGraph::from_ogdl_graph(&calculation)?;
		Self::new_with_metrics(graph, iterations, domains, metrics)
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProgramErrorKind {
	InvalidIterationDomain,
	UnknownKernel,
	DuplicateKernel,
	MissingKernel,
	UninitializedDependency,
	InvalidMetricId,
	DuplicateMetric,
	UnknownMetricValue,
	InvalidMetricValue,
	UnproducedMetricValue,
	UncoveredMetricDomain,
	InvalidDocument,
	MissingField,
	DuplicateField,
	UnknownField,
	UnexpectedChildren,
	InvalidNumber,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProgramError {
	Contract {
		kind: ProgramErrorKind,
		detail: String,
	},
	Syntax(ParseError),
	Graph(OgdlCodecError),
	Build(GraphError),
}

impl ProgramError {
	fn new(kind: ProgramErrorKind, detail: impl Into<String>) -> Self {
		Self::Contract {
			kind,
			detail: detail.into(),
		}
	}

	#[must_use]
	pub const fn kind(&self) -> Option<ProgramErrorKind> {
		match self {
			Self::Contract { kind, .. } => Some(*kind),
			Self::Syntax(_) | Self::Graph(_) | Self::Build(_) => None,
		}
	}
}

impl fmt::Display for ProgramError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Contract { kind, detail } => write!(formatter, "{kind:?}: {detail}"),
			Self::Syntax(error) => write!(formatter, "invalid program OGDL syntax: {error}"),
			Self::Graph(error) => write!(formatter, "invalid calculation graph: {error}"),
			Self::Build(error) => write!(formatter, "cannot build program OGDL: {error}"),
		}
	}
}

impl std::error::Error for ProgramError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			Self::Syntax(error) => Some(error),
			Self::Graph(error) => Some(error),
			Self::Build(error) => Some(error),
			Self::Contract { .. } => None,
		}
	}
}

impl From<ParseError> for ProgramError {
	fn from(error: ParseError) -> Self { Self::Syntax(error) }
}

impl From<OgdlCodecError> for ProgramError {
	fn from(error: OgdlCodecError) -> Self { Self::Graph(error) }
}

impl From<GraphError> for ProgramError {
	fn from(error: GraphError) -> Self { Self::Build(error) }
}

pub type ProgramResult<T> = Result<T, ProgramError>;

fn append_field(graph: &mut Graph, parent: NodeId, name: &str, value: impl Into<String>) -> ProgramResult<()> {
	let field = graph.append_child(parent, name)?;
	graph.append_child(field, value)?;
	Ok(())
}

fn copy_subtree(
	source: &Graph,
	source_id: NodeId,
	target: &mut Graph,
	parent: Option<NodeId>,
) -> ProgramResult<NodeId> {
	let source_node = node(source, source_id, "copied subtree")?;
	let target_id = match parent {
		Some(parent) => target.append_child(parent, source_node.text())?,
		None => target.append_root(source_node.text())?,
	};
	for child in source_node.children() {
		copy_subtree(source, *child, target, Some(target_id))?;
	}
	Ok(target_id)
}

fn node<'a>(graph: &'a Graph, id: NodeId, path: &str) -> ProgramResult<&'a recipe_ogdl::Node> {
	graph.node(id).ok_or_else(|| {
		ProgramError::new(
			ProgramErrorKind::InvalidDocument,
			format!("{path} references an absent node"),
		)
	})
}

fn require_text(graph: &Graph, id: NodeId, expected: &str, path: &str) -> ProgramResult<()> {
	let actual = node(graph, id, path)?.text();
	if actual == expected {
		Ok(())
	} else {
		Err(ProgramError::new(
			ProgramErrorKind::InvalidDocument,
			format!("{path} is {actual:?}, expected {expected:?}"),
		))
	}
}

fn unique_field(graph: &Graph, parent: NodeId, name: &str, path: &str) -> ProgramResult<NodeId> {
	let matches = node(graph, parent, path)?
		.children()
		.iter()
		.copied()
		.filter(|child| node(graph, *child, path).is_ok_and(|child| child.text() == name))
		.collect::<Vec<_>>();
	match matches.as_slice() {
		[field] => Ok(*field),
		[] => {
			Err(ProgramError::new(
				ProgramErrorKind::MissingField,
				format!("{path} is missing field {name:?}"),
			))
		}
		_ => {
			Err(ProgramError::new(
				ProgramErrorKind::DuplicateField,
				format!("{path} repeats field {name:?}"),
			))
		}
	}
}

fn exact_fields(
	graph: &Graph,
	parent: NodeId,
	expected: &[&str],
	path: &str,
) -> ProgramResult<BTreeMap<String, NodeId>> {
	let mut fields = BTreeMap::new();
	for child in node(graph, parent, path)?.children() {
		let child_node = node(graph, *child, path)?;
		let name = child_node.text();
		if !expected.contains(&name) {
			return Err(ProgramError::new(
				ProgramErrorKind::UnknownField,
				format!("{path} contains unknown field {name:?}"),
			));
		}
		if fields.insert(name.to_owned(), *child).is_some() {
			return Err(ProgramError::new(
				ProgramErrorKind::DuplicateField,
				format!("{path} repeats field {name:?}"),
			));
		}
	}
	for field in expected {
		if !fields.contains_key(*field) {
			return Err(ProgramError::new(
				ProgramErrorKind::MissingField,
				format!("{path} is missing field {field:?}"),
			));
		}
	}
	Ok(fields)
}

fn domain_covers(producer: IterationDomain, emission: IterationDomain) -> bool {
	if emission.first_iteration() < producer.first_iteration() {
		return false;
	}
	if !(emission.first_iteration() - producer.first_iteration()).is_multiple_of(producer.stride().get()) {
		return false;
	}
	let emission_last = emission.end_exclusive().map(|end| {
		let span = end - 1 - emission.first_iteration();
		emission.first_iteration() + span / emission.stride().get() * emission.stride().get()
	});
	match (producer.end_exclusive(), emission_last) {
		(Some(producer_end), Some(last)) if last >= producer_end => return false,
		(Some(_), None) => return false,
		(Some(_), Some(_)) | (None, Some(_)) | (None, None) => {}
	}
	emission_last == Some(emission.first_iteration())
		|| emission
			.stride()
			.get()
			.is_multiple_of(producer.stride().get())
}

fn loop_end_text(iterations: LoopIterations) -> String {
	iterations
		.finite()
		.map_or_else(|| "unbounded".to_owned(), |value| value.get().to_string())
}

fn domain_end_text(domain: IterationDomain) -> String {
	domain.end_exclusive()
		.map_or_else(|| "unbounded".to_owned(), |value| value.to_string())
}

fn parse_loop_iterations(value: &str) -> ProgramResult<LoopIterations> {
	if value == "unbounded" {
		return Ok(LoopIterations::UNBOUNDED);
	}
	LoopIterations::new(parse_u64(value, "RecipeProgram.iterations")?).ok_or_else(|| {
		ProgramError::new(
			ProgramErrorKind::InvalidNumber,
			"RecipeProgram.iterations must be nonzero or `unbounded`",
		)
	})
}

fn parse_iteration_domain(first: u64, end_exclusive: &str, stride: u64, path: &str) -> ProgramResult<IterationDomain> {
	let domain = if end_exclusive == "unbounded" {
		IterationDomain::unbounded(first, stride)
	} else {
		IterationDomain::new(
			first,
			parse_u64(end_exclusive, &format!("{path}.end_exclusive"))?,
			stride,
		)
	};
	domain.ok_or_else(|| {
		ProgramError::new(
			ProgramErrorKind::InvalidIterationDomain,
			format!("{path} has an empty, reversed, or zero-stride iteration domain"),
		)
	})
}

fn leaf_value<'a>(graph: &'a Graph, field: NodeId, path: &str) -> ProgramResult<&'a str> {
	let field = node(graph, field, path)?;
	if field.children().len() != 1 {
		return Err(ProgramError::new(
			ProgramErrorKind::UnexpectedChildren,
			format!("{path} requires exactly one value child"),
		));
	}
	let value = node(graph, field.children()[0], path)?;
	if !value.children().is_empty() {
		return Err(ProgramError::new(
			ProgramErrorKind::UnexpectedChildren,
			format!("{path} value must be a leaf"),
		));
	}
	Ok(value.text())
}

fn require_leaf_value(graph: &Graph, field: NodeId, expected: &str, path: &str) -> ProgramResult<()> {
	let actual = leaf_value(graph, field, path)?;
	if actual == expected {
		Ok(())
	} else {
		Err(ProgramError::new(
			ProgramErrorKind::InvalidDocument,
			format!("{path} is {actual:?}, expected {expected:?}"),
		))
	}
}

fn parse_u64(value: &str, path: &str) -> ProgramResult<u64> {
	if value.is_empty() || value.starts_with('+') || value.len() > 1 && value.starts_with('0') {
		return Err(ProgramError::new(
			ProgramErrorKind::InvalidNumber,
			format!("{path} is not a canonical unsigned decimal"),
		));
	}
	value.parse::<u64>().map_err(|error| {
		ProgramError::new(
			ProgramErrorKind::InvalidNumber,
			format!("{path} cannot be represented as u64: {error}"),
		)
	})
}
