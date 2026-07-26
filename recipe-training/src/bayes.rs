//! Typed Bayesian declaration resolution.
//!
//! This module resolves declaration names against a prepared dataset and
//! validates graph structure. It deliberately emits no calculation graph,
//! parameters, training plan, checkpoint, or inference program.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use recipe_ingest::{PreparedDataset, VectorSchema};

/// One Bayesian child declaration in user order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesianDependency {
	child: Vec<u8>,
	parents: Vec<Vec<u8>>,
}

impl BayesianDependency {
	#[must_use]
	pub fn new<C, I, P>(child: C, parents: I) -> Self
	where
		C: AsRef<[u8]>,
		I: IntoIterator<Item = P>,
		P: AsRef<[u8]>,
	{
		Self {
			child: child.as_ref().to_vec(),
			parents: parents
				.into_iter()
				.map(|parent| parent.as_ref().to_vec())
				.collect(),
		}
	}

	#[must_use]
	pub fn child(&self) -> &[u8] {
		&self.child
	}

	#[must_use]
	pub fn parents(&self) -> &[Vec<u8>] {
		&self.parents
	}
}

/// Stable node identity within one resolved schema.
///
/// Identities are assigned by ascending node-name bytes and are independent of
/// declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BayesianNodeId(usize);

impl BayesianNodeId {
	#[must_use]
	pub const fn index(self) -> usize {
		self.0
	}
}

/// Whether a Bayesian node is backed by prepared observations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BayesianNodeSource {
	/// The complete row-free schema retained by ingestion.
	Observed(VectorSchema),
	/// An absent node with no declared parents.
	LatentRoot,
	/// An absent child with one or more declared parents.
	LatentConditional,
}

/// One canonical node in a resolved Bayesian schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesianNodeSchema {
	id: BayesianNodeId,
	name: Vec<u8>,
	source: BayesianNodeSource,
}

impl BayesianNodeSchema {
	#[must_use]
	pub const fn id(&self) -> BayesianNodeId {
		self.id
	}

	#[must_use]
	pub fn name(&self) -> &[u8] {
		&self.name
	}

	#[must_use]
	pub const fn source(&self) -> &BayesianNodeSource {
		&self.source
	}

	#[must_use]
	pub const fn observed_schema(&self) -> Option<&VectorSchema> {
		match &self.source {
			BayesianNodeSource::Observed(schema) => Some(schema),
			BayesianNodeSource::LatentRoot | BayesianNodeSource::LatentConditional => None,
		}
	}

	#[must_use]
	pub const fn is_latent_root(&self) -> bool {
		matches!(self.source, BayesianNodeSource::LatentRoot)
	}

	#[must_use]
	pub const fn is_latent(&self) -> bool {
		matches!(
			self.source,
			BayesianNodeSource::LatentRoot | BayesianNodeSource::LatentConditional
		)
	}
}

/// One resolved declaration. Entries remain in repeated-call order and each
/// `parents` list remains in its original order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBayesianDependency {
	declaration_index: usize,
	child: BayesianNodeId,
	parents: Vec<BayesianNodeId>,
}

impl ResolvedBayesianDependency {
	#[must_use]
	pub const fn declaration_index(&self) -> usize {
		self.declaration_index
	}

	#[must_use]
	pub const fn child(&self) -> BayesianNodeId {
		self.child
	}

	#[must_use]
	pub fn parents(&self) -> &[BayesianNodeId] {
		&self.parents
	}
}

/// A validated, row-free Bayesian schema.
///
/// `nodes` use canonical name order, `declarations` use user order, and
/// `execution_order` is a separately derived deterministic topological order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBayesianSchema {
	nodes: Vec<BayesianNodeSchema>,
	declarations: Vec<ResolvedBayesianDependency>,
	execution_order: Vec<BayesianNodeId>,
}

impl ResolvedBayesianSchema {
	#[must_use]
	pub fn nodes(&self) -> &[BayesianNodeSchema] {
		&self.nodes
	}

	#[must_use]
	pub fn declarations(&self) -> &[ResolvedBayesianDependency] {
		&self.declarations
	}

	#[must_use]
	pub fn execution_order(&self) -> &[BayesianNodeId] {
		&self.execution_order
	}

	#[must_use]
	pub fn node(&self, id: BayesianNodeId) -> Option<&BayesianNodeSchema> {
		self.nodes.get(id.index())
	}

	#[must_use]
	pub fn node_id(&self, name: impl AsRef<[u8]>) -> Option<BayesianNodeId> {
		let name = name.as_ref();
		self.nodes
			.binary_search_by(|node| node.name.as_slice().cmp(name))
			.ok()
			.map(BayesianNodeId)
	}
}

/// Typed reason a Bayesian schema could not be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BayesianSchemaErrorKind {
	EmptyName,
	DuplicateDatasetName,
	DuplicateChild,
	DuplicateParent,
	SelfDependency,
	Cycle,
}

/// One typed segment in an error's structural path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BayesianSchemaPathSegment {
	Dataset,
	Vectors,
	Vector(usize),
	Name,
	Declarations,
	Declaration(usize),
	Child,
	Parents,
	Parent(usize),
	Graph,
	ExecutionOrder,
}

/// A structural Bayesian resolution error with a machine-readable hierarchy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesianSchemaError {
	kind: BayesianSchemaErrorKind,
	path: Vec<BayesianSchemaPathSegment>,
	detail: String,
}

impl BayesianSchemaError {
	fn new(
		kind: BayesianSchemaErrorKind,
		path: impl Into<Vec<BayesianSchemaPathSegment>>,
		detail: impl Into<String>,
	) -> Self {
		Self {
			kind,
			path: path.into(),
			detail: detail.into(),
		}
	}

	#[must_use]
	pub const fn kind(&self) -> BayesianSchemaErrorKind {
		self.kind
	}

	#[must_use]
	pub fn path(&self) -> &[BayesianSchemaPathSegment] {
		&self.path
	}

	#[must_use]
	pub fn detail(&self) -> &str {
		&self.detail
	}
}

impl fmt::Display for BayesianSchemaError {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			formatter,
			"{:?} at {}: {}",
			self.kind,
			DisplayPath(&self.path),
			self.detail
		)
	}
}

impl std::error::Error for BayesianSchemaError {}

struct DisplayPath<'a>(&'a [BayesianSchemaPathSegment]);

impl fmt::Display for DisplayPath<'_> {
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		for segment in self.0 {
			match segment {
				BayesianSchemaPathSegment::Dataset => formatter.write_str("dataset")?,
				BayesianSchemaPathSegment::Vectors => formatter.write_str(".vectors")?,
				BayesianSchemaPathSegment::Vector(vector) => write!(formatter, "[{vector}]")?,
				BayesianSchemaPathSegment::Name => formatter.write_str(".name")?,
				BayesianSchemaPathSegment::Declarations => formatter.write_str("declarations")?,
				BayesianSchemaPathSegment::Declaration(declaration) => write!(formatter, "[{declaration}]")?,
				BayesianSchemaPathSegment::Child => formatter.write_str(".child")?,
				BayesianSchemaPathSegment::Parents => formatter.write_str(".parents")?,
				BayesianSchemaPathSegment::Parent(parent) => write!(formatter, "[{parent}]")?,
				BayesianSchemaPathSegment::Graph => formatter.write_str("graph")?,
				BayesianSchemaPathSegment::ExecutionOrder => formatter.write_str(".execution-order")?,
			}
		}
		Ok(())
	}
}

pub type BayesianSchemaResult<T> = Result<T, BayesianSchemaError>;

/// Resolve declarations against prepared vector schemas without compiling or
/// executing a Bayesian model.
///
/// Every prepared vector becomes an observed node, including vectors omitted
/// from the declarations. An absent parent-only or zero-indegree name becomes
/// a latent root; an absent child with parents becomes a latent conditional
/// node. Absence of observations is not a structural error.
///
/// # Errors
///
/// Returns a typed structural error for empty or duplicate names, duplicate
/// edges, self-dependencies, or dependency cycles.
pub fn resolve_bayesian_schema(
	dataset: &PreparedDataset,
	dependencies: &[BayesianDependency],
) -> BayesianSchemaResult<ResolvedBayesianSchema> {
	let observed = observed_schemas(dataset)?;
	validate_dependencies(dependencies)?;

	let mut sources = observed
		.into_iter()
		.map(|(name, schema)| (name, BayesianNodeSource::Observed(schema)))
		.collect::<BTreeMap<_, _>>();
	for dependency in dependencies {
		match sources.entry(dependency.child.clone()) {
			std::collections::btree_map::Entry::Vacant(entry) => {
				entry.insert(if dependency.parents.is_empty() {
					BayesianNodeSource::LatentRoot
				} else {
					BayesianNodeSource::LatentConditional
				});
			}
			std::collections::btree_map::Entry::Occupied(mut entry) => {
				if !dependency.parents.is_empty() && matches!(entry.get(), BayesianNodeSource::LatentRoot) {
					entry.insert(BayesianNodeSource::LatentConditional);
				}
			}
		}
		for parent in &dependency.parents {
			sources
				.entry(parent.clone())
				.or_insert(BayesianNodeSource::LatentRoot);
		}
	}

	let nodes = sources
		.into_iter()
		.enumerate()
		.map(|(index, (name, source))| BayesianNodeSchema {
			id: BayesianNodeId(index),
			name,
			source,
		})
		.collect::<Vec<_>>();
	let ids = nodes
		.iter()
		.map(|node| (node.name.clone(), node.id))
		.collect::<BTreeMap<_, _>>();
	let declarations = dependencies
		.iter()
		.enumerate()
		.map(
			|(declaration_index, dependency)| ResolvedBayesianDependency {
				declaration_index,
				child: ids[&dependency.child],
				parents: dependency
					.parents
					.iter()
					.map(|parent| ids[parent])
					.collect(),
			},
		)
		.collect::<Vec<_>>();
	let execution_order = deterministic_topological_order(&nodes, &declarations)?;

	Ok(ResolvedBayesianSchema {
		nodes,
		declarations,
		execution_order,
	})
}

fn observed_schemas(dataset: &PreparedDataset) -> BayesianSchemaResult<BTreeMap<Vec<u8>, VectorSchema>> {
	let mut observed = BTreeMap::new();
	for (index, vector) in dataset.vectors().iter().enumerate() {
		if observed
			.insert(vector.name().to_vec(), vector.schema())
			.is_some()
		{
			return Err(BayesianSchemaError::new(
				BayesianSchemaErrorKind::DuplicateDatasetName,
				[
					BayesianSchemaPathSegment::Dataset,
					BayesianSchemaPathSegment::Vectors,
					BayesianSchemaPathSegment::Vector(index),
					BayesianSchemaPathSegment::Name,
				],
				format!(
					"prepared vector name {:?} appears more than once",
					String::from_utf8_lossy(vector.name())
				),
			));
		}
	}
	Ok(observed)
}

fn validate_dependencies(dependencies: &[BayesianDependency]) -> BayesianSchemaResult<()> {
	let mut children = BTreeMap::<&[u8], usize>::new();
	for (declaration_index, dependency) in dependencies.iter().enumerate() {
		let declaration_path = || {
			vec![
				BayesianSchemaPathSegment::Declarations,
				BayesianSchemaPathSegment::Declaration(declaration_index),
			]
		};
		if dependency.child.is_empty() {
			let mut path = declaration_path();
			path.push(BayesianSchemaPathSegment::Child);
			return Err(BayesianSchemaError::new(
				BayesianSchemaErrorKind::EmptyName,
				path,
				"Bayesian child name is empty",
			));
		}
		if let Some(first) = children.insert(&dependency.child, declaration_index) {
			let mut path = declaration_path();
			path.push(BayesianSchemaPathSegment::Child);
			return Err(BayesianSchemaError::new(
				BayesianSchemaErrorKind::DuplicateChild,
				path,
				format!(
					"Bayesian child {:?} was already declared at declarations[{first}]",
					String::from_utf8_lossy(&dependency.child)
				),
			));
		}

		let mut parents = BTreeMap::<&[u8], usize>::new();
		for (parent_index, parent) in dependency.parents.iter().enumerate() {
			let parent_path = || {
				let mut path = declaration_path();
				path.extend([
					BayesianSchemaPathSegment::Parents,
					BayesianSchemaPathSegment::Parent(parent_index),
				]);
				path
			};
			if parent.is_empty() {
				return Err(BayesianSchemaError::new(
					BayesianSchemaErrorKind::EmptyName,
					parent_path(),
					"Bayesian parent name is empty",
				));
			}
			if parent == &dependency.child {
				return Err(BayesianSchemaError::new(
					BayesianSchemaErrorKind::SelfDependency,
					parent_path(),
					format!(
						"Bayesian node {:?} cannot name itself as a parent",
						String::from_utf8_lossy(parent)
					),
				));
			}
			if let Some(first) = parents.insert(parent, parent_index) {
				return Err(BayesianSchemaError::new(
					BayesianSchemaErrorKind::DuplicateParent,
					parent_path(),
					format!(
						"Bayesian parent {:?} was already declared at parents[{first}]",
						String::from_utf8_lossy(parent)
					),
				));
			}
		}
	}
	Ok(())
}

fn deterministic_topological_order(
	nodes: &[BayesianNodeSchema],
	declarations: &[ResolvedBayesianDependency],
) -> BayesianSchemaResult<Vec<BayesianNodeId>> {
	let mut outgoing = vec![Vec::new(); nodes.len()];
	let mut indegree = vec![0usize; nodes.len()];
	for declaration in declarations {
		for parent in &declaration.parents {
			outgoing[parent.index()].push(declaration.child);
			indegree[declaration.child.index()] += 1;
		}
	}
	let mut ready = indegree
		.iter()
		.enumerate()
		.filter_map(|(index, degree)| (*degree == 0).then_some(BayesianNodeId(index)))
		.collect::<BTreeSet<_>>();
	let mut order = Vec::with_capacity(nodes.len());
	while let Some(node) = ready.iter().next().copied() {
		ready.remove(&node);
		order.push(node);
		for child in &outgoing[node.index()] {
			indegree[child.index()] -= 1;
			if indegree[child.index()] == 0 {
				ready.insert(*child);
			}
		}
	}
	if order.len() != nodes.len() {
		let names = indegree
			.iter()
			.enumerate()
			.filter(|(_, degree)| **degree != 0)
			.map(|(index, _)| String::from_utf8_lossy(nodes[index].name()).into_owned())
			.collect::<Vec<_>>();
		return Err(BayesianSchemaError::new(
			BayesianSchemaErrorKind::Cycle,
			[
				BayesianSchemaPathSegment::Graph,
				BayesianSchemaPathSegment::ExecutionOrder,
			],
			format!("Bayesian declarations contain a dependency cycle among {names:?}"),
		));
	}
	Ok(order)
}
