#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Static lifecycle envelope for Recipe calculation graphs.
//!
//! [`CalculationGraph`](recipe_language::CalculationGraph) remains an acyclic
//! description of calculation dependencies. This crate assigns those kernels
//! to explicit subsets of a finite or gracefully stopped unbounded loop without unrolling the graph or its
//! artifacts. Init admission and exit remain separate runtime phases.

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};
use core::{error::Error, fmt};

use recipe_core::{ByteCount, KernelTemplateId, MetricId, ValueId};
pub use recipe_core::{IterationDomain, LoopIterations};
use recipe_language::{CalculationGraph, OgdlCodecError};
use recipe_ogdl::{Graph, GraphError, NodeId, ParseError};

/// Root node name of a serialized Recipe program.
const PROGRAM_ROOT: &str = "RecipeProgram";
/// Schema name stored beneath the serialized program root.
const PROGRAM_SCHEMA: &str = "StaticCalculationProgram";
/// Current serialized Recipe program version.
const PROGRAM_VERSION: &str = "2";
/// Earlier serialized version accepted by the decoder.
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
	/// Acyclic calculation graph executed by the program.
	graph: CalculationGraph,
	/// Declared loop horizon.
	iterations: LoopIterations,
	/// Activation domain for each calculation kernel.
	domains: Vec<KernelIterationDomain>,
	/// User-visible scalar emissions and their activation domains.
	metrics: Vec<MetricEmission>,
}

impl StaticCalculationProgram {
	/// Creates a static program without metric emissions.
	///
	/// # Errors
	///
	/// Returns an error when the graph or any kernel activation domain is
	/// inconsistent with the declared loop horizon.
	#[inline]
	pub fn new<I: Into<LoopIterations>>(
		graph: CalculationGraph,
		iterations: I,
		domains: Vec<KernelIterationDomain>,
	) -> ProgramResult<Self> {
		return Self::new_with_metrics(graph, iterations, domains, Vec::new());
	}

	/// Creates a static program with explicit kernel and metric domains.
	///
	/// # Errors
	///
	/// Returns an error when the graph, a kernel domain, or a metric emission is
	/// inconsistent with the declared loop horizon.
	#[inline]
	pub fn new_with_metrics<I: Into<LoopIterations>>(
		graph: CalculationGraph,
		iterations: I,
		mut domains: Vec<KernelIterationDomain>,
		mut metrics: Vec<MetricEmission>,
	) -> ProgramResult<Self> {
		let loop_iterations = iterations.into();
		domains.sort_by_key(|entry| return entry.kernel);
		metrics.sort_by_key(|entry| return entry.metric);
		let program = Self {
			graph,
			iterations: loop_iterations,
			domains,
			metrics,
		};
		program.validate()?;
		return Ok(program);
	}

	/// Creates a program that activates every kernel on every loop iteration.
	///
	/// # Errors
	///
	/// Returns an error when the graph cannot form a valid static program for
	/// the declared loop horizon.
	#[inline]
	pub fn every_iteration<I: Into<LoopIterations>>(graph: CalculationGraph, iterations: I) -> ProgramResult<Self> {
		let loop_iterations = iterations.into();
		let domains = graph
			.nodes
			.iter()
			.map(|node| {
				return KernelIterationDomain {
					kernel: node.kernel.id,
					domain: IterationDomain::every(loop_iterations),
				};
			})
			.collect();
		return Self::new(graph, loop_iterations, domains);
	}

	/// Replaces the program's metric emissions.
	///
	/// # Errors
	///
	/// Returns an error when a metric emission is duplicated, references an
	/// unknown value, or has a domain outside the program loop horizon.
	#[inline]
	pub fn with_metrics(mut self, mut metrics: Vec<MetricEmission>) -> ProgramResult<Self> {
		metrics.sort_by_key(|entry| return entry.metric);
		self.metrics = metrics;
		self.validate()?;
		return Ok(self);
	}

	/// Validates the graph, activation domains, and metric emissions.
	///
	/// # Errors
	///
	/// Returns an error for an invalid graph, missing or duplicate kernel
	/// domains, incompatible dependency domains, or invalid metric emissions.
	#[inline]
	pub fn validate(&self) -> ProgramResult<()> {
		self.graph
			.validate()
			.map_err(|error| return ProgramError::Graph(OgdlCodecError::InvalidGraph(error)))?;
		let kernels = self
			.graph
			.nodes
			.iter()
			.map(|node| return node.kernel.id)
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
		if let Some(missing) = kernels.difference(&assigned).next().copied() {
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
				return node.kernel
					.outputs
					.iter()
					.map(move |output| return (*output, node.kernel.id));
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
			.map(|tensor| return (tensor.id, tensor))
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
				return ProgramError::new(
					ProgramErrorKind::UnknownMetricValue,
					format!(
						"metric {} references unknown value {}",
						emission.metric, emission.value
					),
				);
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
				return ProgramError::new(
					ProgramErrorKind::UnproducedMetricValue,
					format!(
						"metric {} value {} has no calculation producer",
						emission.metric, emission.value
					),
				);
			})?;
			let producer_domain = domain_by_kernel[&producer];
			let emission_domain = emission.domain;
			let producer_covers_emission = if emission_domain.first_iteration() < producer_domain.first_iteration()
				|| !(emission_domain.first_iteration() - producer_domain.first_iteration())
					.is_multiple_of(producer_domain.stride().get())
			{
				false
			} else {
				let emission_last = emission_domain.end_exclusive().map(|end| {
					let span = end - 1 - emission_domain.first_iteration();
					return emission_domain.first_iteration()
						+ span / emission_domain.stride().get() * emission_domain.stride().get();
				});
				match (producer_domain.end_exclusive(), emission_last) {
					(Some(producer_end), Some(last)) if last >= producer_end => false,
					(Some(_), None) => false,
					(Some(_) | None, Some(_)) | (None, None) => {
						emission_last == Some(emission_domain.first_iteration())
							|| emission_domain
								.stride()
								.get()
								.is_multiple_of(producer_domain.stride().get())
					}
				}
			};
			if !producer_covers_emission {
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
		return Ok(());
	}

	#[must_use]
	#[inline]
	pub const fn graph(&self) -> &CalculationGraph { return &self.graph; }

	#[must_use]
	#[inline]
	pub const fn iterations(&self) -> LoopIterations { return self.iterations; }

	#[must_use]
	#[inline]
	pub fn domains(&self) -> &[KernelIterationDomain] { return &self.domains; }

	#[must_use]
	#[inline]
	pub fn domain(&self, kernel: KernelTemplateId) -> Option<IterationDomain> {
		return self.domains
			.binary_search_by_key(&kernel, |entry| return entry.kernel)
			.ok()
			.map(|index| return self.domains[index].domain);
	}

	#[must_use]
	#[inline]
	pub fn metrics(&self) -> &[MetricEmission] { return &self.metrics; }

	#[must_use]
	#[inline]
	pub fn metric(&self, metric: MetricId) -> Option<MetricEmission> {
		return self.metrics
			.binary_search_by_key(&metric, |entry| return entry.metric)
			.ok()
				.map(|index| return self.metrics[index]);
	}

	/// Serializes the validated program as canonical OGDL text.
	///
	/// # Errors
	///
	/// Returns an error when the program is invalid or its OGDL graph cannot be
	/// constructed.
	#[inline]
	pub fn to_ogdl(&self) -> ProgramResult<String> {
		self.validate()?;
		let graph = self.to_ogdl_graph()?;
		return Ok(graph.to_canonical_string());
	}

	/// Builds the canonical OGDL graph for this program.
	///
	/// # Errors
	///
	/// Returns an error when the program is invalid or an OGDL node cannot be
	/// appended.
	#[inline]
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
			return ProgramError::new(
				ProgramErrorKind::InvalidDocument,
				"calculation graph OGDL has no root",
			);
		})?;
		copy_subtree(&calculation, calculation_root, &mut output, None)?;
		return Ok(output);
	}

	/// Parses a static program from OGDL text.
	///
	/// # Errors
	///
	/// Returns an error when the text is invalid OGDL or does not encode a valid
	/// static program.
	#[inline]
	pub fn from_ogdl(input: &str) -> ProgramResult<Self> {
		let graph = Graph::parse(input)?;
		return Self::from_ogdl_graph(&graph);
	}

	/// Decodes a static program from an OGDL graph.
	///
	/// # Errors
	///
	/// Returns an error when required roots or fields are absent, duplicated, or
	/// malformed, or when the decoded program fails validation.
	#[inline]
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
		let version_fields = node(source, program_root, "RecipeProgram")?
			.children()
			.iter()
			.copied()
			.filter(|child_id| {
				return node(source, *child_id, "RecipeProgram")
					.is_ok_and(|child_node| return child_node.text() == "version");
			})
			.collect::<Vec<_>>();
		if version_fields.len() > 1 {
			return Err(ProgramError::new(
				ProgramErrorKind::DuplicateField,
				"RecipeProgram repeats field \"version\"",
			));
		}
		let Some(version_field) = version_fields.first().copied() else {
			return Err(ProgramError::new(
				ProgramErrorKind::MissingField,
				"RecipeProgram is missing field \"version\"",
			));
		};
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
		let schema = leaf_value(source, fields["schema"], "RecipeProgram.schema")?;
		if schema != PROGRAM_SCHEMA {
			return Err(ProgramError::new(
				ProgramErrorKind::InvalidDocument,
				format!(
					"RecipeProgram.schema is {schema:?}, expected {PROGRAM_SCHEMA:?}"
				),
			));
		}
		let iteration_text = leaf_value(
			source,
			fields["iterations"],
			"RecipeProgram.iterations",
		)?;
		let iterations = if iteration_text == "unbounded" {
			LoopIterations::UNBOUNDED
		} else {
			LoopIterations::new(parse_u64(iteration_text, "RecipeProgram.iterations")?).ok_or_else(
				|| {
					return ProgramError::new(
						ProgramErrorKind::InvalidNumber,
						"RecipeProgram.iterations must be nonzero or `unbounded`",
					);
				},
			)?
		};
		let domains_node = fields["domains"];
		let domain_children = node(source, domains_node, "RecipeProgram.domains")?.children();
		let mut domains = Vec::with_capacity(domain_children.len());
		for (index, child) in domain_children.iter().copied().enumerate() {
			let path = format!("RecipeProgram.domains[{index}]");
			require_text(source, child, "domain", &path)?;
			let domain_fields = exact_fields(
				source,
				child,
				&["kernel", "first", "end_exclusive", "stride"],
				&path,
			)?;
			let kernel = KernelTemplateId::new(parse_u64(
				leaf_value(source, domain_fields["kernel"], &format!("{path}.kernel"))?,
				&format!("{path}.kernel"),
			)?);
			let first = parse_u64(
				leaf_value(source, domain_fields["first"], &format!("{path}.first"))?,
				&format!("{path}.first"),
			)?;
			let end_exclusive = leaf_value(
				source,
				domain_fields["end_exclusive"],
				&format!("{path}.end_exclusive"),
			)?;
			let stride = parse_u64(
				leaf_value(source, domain_fields["stride"], &format!("{path}.stride"))?,
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
				let metric_fields = exact_fields(
					source,
					child,
					&["id", "value", "first", "end_exclusive", "stride"],
					&path,
				)?;
				let metric = MetricId::new(parse_u64(
					leaf_value(source, metric_fields["id"], &format!("{path}.id"))?,
					&format!("{path}.id"),
				)?);
				let value = ValueId::new(parse_u64(
					leaf_value(source, metric_fields["value"], &format!("{path}.value"))?,
					&format!("{path}.value"),
				)?);
				let first = parse_u64(
					leaf_value(source, metric_fields["first"], &format!("{path}.first"))?,
					&format!("{path}.first"),
				)?;
				let end_exclusive = leaf_value(
					source,
					metric_fields["end_exclusive"],
					&format!("{path}.end_exclusive"),
				)?;
				let stride = parse_u64(
					leaf_value(source, metric_fields["stride"], &format!("{path}.stride"))?,
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
		return Self::new_with_metrics(graph, iterations, domains, metrics);
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

impl ProgramErrorKind {
	/// Returns the stable text used when displaying this error kind.
	#[must_use]
	#[inline]
	const fn label(self) -> &'static str {
		return match self {
			Self::InvalidIterationDomain => "invalid iteration domain",
			Self::UnknownKernel => "unknown kernel",
			Self::DuplicateKernel => "duplicate kernel",
			Self::MissingKernel => "missing kernel",
			Self::UninitializedDependency => "uninitialized dependency",
			Self::InvalidMetricId => "invalid metric ID",
			Self::DuplicateMetric => "duplicate metric",
			Self::UnknownMetricValue => "unknown metric value",
			Self::InvalidMetricValue => "invalid metric value",
			Self::UnproducedMetricValue => "unproduced metric value",
			Self::UncoveredMetricDomain => "uncovered metric domain",
			Self::InvalidDocument => "invalid document",
			Self::MissingField => "missing field",
			Self::DuplicateField => "duplicate field",
			Self::UnknownField => "unknown field",
			Self::UnexpectedChildren => "unexpected children",
			Self::InvalidNumber => "invalid number",
		};
	}
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
	/// Creates a program contract error with structured kind and detail.
	fn new(kind: ProgramErrorKind, detail: impl Into<String>) -> Self {
		return Self::Contract {
			kind,
			detail: detail.into(),
		};
	}

	#[must_use]
	#[inline]
	pub const fn kind(&self) -> Option<ProgramErrorKind> {
		return match self {
			&Self::Contract { kind, .. } => Some(kind),
			&Self::Syntax(_) | &Self::Graph(_) | &Self::Build(_) => None,
		};
	}
}

impl fmt::Display for ProgramError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		return match self.clone() {
			Self::Contract { kind, detail } => write!(f, "{}: {detail}", kind.label()),
			Self::Syntax(error) => write!(f, "invalid program OGDL syntax: {error}"),
			Self::Graph(error) => write!(f, "invalid calculation graph: {error}"),
			Self::Build(error) => write!(f, "cannot build program OGDL: {error}"),
		};
	}
}

impl Error for ProgramError {}

impl From<ParseError> for ProgramError {
	#[inline]
	fn from(error: ParseError) -> Self { return Self::Syntax(error); }
}

impl From<OgdlCodecError> for ProgramError {
	#[inline]
	fn from(error: OgdlCodecError) -> Self { return Self::Graph(error); }
}

impl From<GraphError> for ProgramError {
	#[inline]
	fn from(error: GraphError) -> Self { return Self::Build(error); }
}

pub type ProgramResult<T> = Result<T, ProgramError>;

/// Appends one named field with one leaf value to an OGDL graph.
fn append_field(graph: &mut Graph, parent: NodeId, name: &str, value: impl Into<String>) -> ProgramResult<()> {
	let field = graph.append_child(parent, name)?;
	graph.append_child(field, value)?;
	return Ok(());
}

/// Copies one source subtree into a target graph and returns its new root.
///
/// # Errors
///
/// Returns an error when a referenced source node is absent or the target
/// graph rejects an appended node.
fn copy_subtree(
	source: &Graph,
	source_id: NodeId,
	target: &mut Graph,
	parent_id: Option<NodeId>,
) -> ProgramResult<NodeId> {
	let source_node = node(source, source_id, "copied subtree")?;
	let target_id = match parent_id {
		Some(parent_node) => target.append_child(parent_node, source_node.text())?,
		None => target.append_root(source_node.text())?,
	};
	for child in source_node.children() {
		copy_subtree(source, *child, target, Some(target_id))?;
	}
	return Ok(target_id);
}

/// Resolves one node ID and attaches its logical path to absence errors.
///
/// # Errors
///
/// Returns an error when `id` is absent from `graph`.
fn node<'a>(graph: &'a Graph, id: NodeId, path: &str) -> ProgramResult<&'a recipe_ogdl::Node> {
	return graph.node(id).ok_or_else(|| {
		return ProgramError::new(
			ProgramErrorKind::InvalidDocument,
			format!("{path} references an absent node"),
		);
	});
}

/// Requires one node to contain the expected text.
///
/// # Errors
///
/// Returns an error when the node is absent or its text differs.
fn require_text(graph: &Graph, id: NodeId, expected: &str, path: &str) -> ProgramResult<()> {
	let actual = node(graph, id, path)?.text();
	if actual == expected {
		return Ok(());
	}
	return Err(ProgramError::new(
		ProgramErrorKind::InvalidDocument,
		format!("{path} is {actual:?}, expected {expected:?}"),
	));
}

/// Collects exactly the named child fields beneath one OGDL node.
///
/// # Errors
///
/// Returns an error for absent, duplicate, or unexpected fields.
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
	return Ok(fields);
}

/// Formats the finite loop bound or the unbounded sentinel.
fn loop_end_text(iterations: LoopIterations) -> String {
	return iterations
		.finite()
		.map_or_else(|| return "unbounded".to_owned(), |value| return value.get().to_string());
}

/// Formats the finite domain end or the unbounded sentinel.
fn domain_end_text(domain: IterationDomain) -> String {
	return domain
		.end_exclusive()
		.map_or_else(|| return "unbounded".to_owned(), |value| return value.to_string());
}

/// Parses one finite or unbounded iteration domain.
///
/// # Errors
///
/// Returns an error when the end or stride is malformed or the resulting
/// domain is empty, reversed, or has zero stride.
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
	return domain.ok_or_else(|| {
		return ProgramError::new(
			ProgramErrorKind::InvalidIterationDomain,
			format!("{path} has an empty, reversed, or zero-stride iteration domain"),
		);
	});
}

/// Reads the sole leaf value beneath one field node.
///
/// # Errors
///
/// Returns an error when the field is absent, has multiple values, or its
/// value has children.
fn leaf_value<'a>(graph: &'a Graph, field_id: NodeId, path: &str) -> ProgramResult<&'a str> {
	let field_node = node(graph, field_id, path)?;
	if field_node.children().len() != 1 {
		return Err(ProgramError::new(
			ProgramErrorKind::UnexpectedChildren,
			format!("{path} requires exactly one value child"),
		));
	}
	let value = node(graph, field_node.children()[0], path)?;
	if !value.children().is_empty() {
		return Err(ProgramError::new(
			ProgramErrorKind::UnexpectedChildren,
			format!("{path} value must be a leaf"),
		));
	}
	return Ok(value.text());
}

/// Parses one canonical unsigned decimal value.
///
/// # Errors
///
/// Returns an error for an empty, signed, zero-padded, or out-of-range value.
fn parse_u64(value: &str, path: &str) -> ProgramResult<u64> {
	if value.is_empty() || value.starts_with('+') || value.len() > 1 && value.starts_with('0') {
		return Err(ProgramError::new(
			ProgramErrorKind::InvalidNumber,
			format!("{path} is not a canonical unsigned decimal"),
		));
	}
	return value.parse::<u64>().map_err(|error| {
		return ProgramError::new(
			ProgramErrorKind::InvalidNumber,
			format!("{path} cannot be represented as u64: {error}"),
		);
	});
}
