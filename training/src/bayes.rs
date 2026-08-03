//! Typed Bayesian declaration resolution.
//!
//! This module resolves declaration names against a prepared dataset and
//! validates graph structure. It deliberately emits no calculation graph,
//! parameters, training plan, checkpoint, or inference program.

use core::{error::Error, fmt, iter::once}; use alloc::collections::{BTreeMap, BTreeSet, btree_map::Entry};

use recipe_ingest::{ PreparedDataset, PreparedVector, SemanticType, VectorEncoding, VectorMetadata, VectorRole,
	VectorSchema, };

use crate::{TrainingCompileError, TrainingCompileErrorKind, TrainingCompileResult};

pub const CATEGORICAL_BAYES_SMOOTHING: f32 = 1.0;

/// One Bayesian child declaration in user order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesianDependency {
	/// Declared child vector name bytes.
	child: Vec<u8>,
	/// Declared parent vector name bytes in user order.
	parents: Vec<Vec<u8>>, }

impl BayesianDependency {
	#[must_use]
	#[inline]
	pub fn new<C, I, P>(child: C, parents: I) -> Self
	where C: AsRef<[u8]>, I: IntoIterator<Item = P>, P: AsRef<[u8]>, { return Self { child: child.as_ref().to_vec(),
			parents: parents .into_iter() .map(|parent| { return parent.as_ref().to_vec(); }) .collect(), }; }

	#[must_use]
	#[inline]
	pub fn child(&self) -> &[u8] { return &self.child; }

	#[must_use]
	#[inline]
	pub fn parents(&self) -> &[Vec<u8>] { return &self.parents; } }

/// Stable node identity within one resolved schema.
///
/// Identities are assigned by ascending node-name bytes and are independent of
/// declaration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BayesianNodeId(usize);

impl BayesianNodeId {
	#[must_use]
	#[inline]
	pub const fn index(self) -> usize { return self.0; } }

/// Whether a Bayesian node is backed by prepared observations.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BayesianNodeSource {
	/// The complete row-free schema retained by ingestion.
	Observed(VectorSchema),
	/// An absent node with no declared parents.
	LatentRoot,
	/// An absent child with one or more declared parents.
	LatentConditional, }

/// One canonical node in a resolved Bayesian schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesianNodeSchema {
	/// Stable identity assigned by canonical name order.
	id: BayesianNodeId,
	/// Original node name bytes.
	name: Vec<u8>,
	/// Prepared observation or latent-node classification.
	source: BayesianNodeSource, }

impl BayesianNodeSchema {
	#[must_use]
	#[inline]
	pub const fn id(&self) -> BayesianNodeId { return self.id; }

	#[must_use]
	#[inline]
	pub fn name(&self) -> &[u8] { return &self.name; }

	#[must_use]
	#[inline]
	pub const fn source(&self) -> &BayesianNodeSource { return &self.source; }


	#[must_use]
	#[inline]
	pub const fn is_latent(&self) -> bool { return matches!( self.source,
			BayesianNodeSource::LatentRoot | BayesianNodeSource::LatentConditional ); } }

/// One resolved declaration. Entries remain in repeated-call order and each
/// `parents` list remains in its original order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBayesianDependency {
	/// Position of the originating declaration in user order.
	declaration_index: usize,
	/// Canonical identity of the declared child.
	child: BayesianNodeId,
	/// Canonical parent identities in declaration order.
	parents: Vec<BayesianNodeId>, }

impl ResolvedBayesianDependency {
	#[must_use]
	#[inline]
	pub const fn declaration_index(&self) -> usize { return self.declaration_index; }

	#[must_use]
	#[inline]
	pub const fn child(&self) -> BayesianNodeId { return self.child; }

	#[must_use]
	#[inline]
	pub fn parents(&self) -> &[BayesianNodeId] { return &self.parents; } }

/// A validated, row-free Bayesian schema.
///
/// `nodes` use canonical name order, `declarations` use user order, and
/// `execution_order` is a separately derived deterministic topological order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedBayesianSchema {
	/// Nodes in canonical ascending name order.
	nodes: Vec<BayesianNodeSchema>,
	/// Resolved dependencies in user declaration order.
	declarations: Vec<ResolvedBayesianDependency>,
	/// Deterministic topological execution order.
	execution_order: Vec<BayesianNodeId>, }

impl ResolvedBayesianSchema {
	#[must_use]
	#[inline]
	pub fn nodes(&self) -> &[BayesianNodeSchema] { return &self.nodes; }

	#[must_use]
	#[inline]
	pub fn declarations(&self) -> &[ResolvedBayesianDependency] { return &self.declarations; }

	#[must_use]
	#[inline]
	pub fn execution_order(&self) -> &[BayesianNodeId] { return &self.execution_order; }

	#[must_use]
	#[inline]
	pub fn node(&self, id: BayesianNodeId) -> Option<&BayesianNodeSchema> { return self.nodes.get(id.index()); }

}

/// Exact categorical identity retained for one observed Bayesian node.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesianCategoricalSchema {
	/// Source-vector position in the prepared dataset.
	source_index: usize,
	/// Original vector name bytes.
	name: Vec<u8>,
	/// Known categorical labels in encoded-code order.
	dictionary: Vec<Vec<u8>>, }

impl BayesianCategoricalSchema {
	/// Reconstruct a categorical schema from its semantic artifact fields.
	#[inline]
	pub(crate) const fn from_parts(source_index: usize, name: Vec<u8>, dictionary: Vec<Vec<u8>>) -> Self { return Self {
			source_index, name, dictionary, }; }

	#[must_use]
	#[inline]
	pub const fn source_index(&self) -> usize { return self.source_index; }

	#[must_use]
	#[inline]
	pub fn name(&self) -> &[u8] { return &self.name; }

	#[must_use]
	#[inline]
	pub fn dictionary(&self) -> &[Vec<u8>] { return &self.dictionary; }

	#[must_use]
	#[inline]
	pub const fn inference_cardinality(&self) -> Option<usize> { return self.dictionary.len().checked_add(1); } }

/// Saved observations for one executable categorical Bayesian conditional.
///
/// One declared categorical target is conditioned on one or more declared
/// categorical feature parents. Codes remain row-major in parent declaration
/// order. The artifact stores observations rather than host-computed counts;
/// histogramming and posterior calculation therefore remain native payload
/// calculations at inference time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesianCategoricalReferenceSet {
	/// Parent schemas in declaration order.
	parents: Vec<BayesianCategoricalSchema>,
	/// Declared child schema.
	child: BayesianCategoricalSchema,
	/// Original source-row identities in retained order.
	reference_source_rows: Vec<usize>,
	/// Number of retained reference rows.
	reference_rows: usize,
	/// Row-major parent dictionary codes.
	parent_codes: Vec<i32>,
	/// Child dictionary code for each retained row.
	child_codes: Vec<i32>,
	/// Parent cardinalities including each unseen-value route.
	parent_cardinalities: Vec<i32>,
	/// Mixed-radix multiplier for each parent.
	parent_multipliers: Vec<i32>,
	/// Total mixed-radix parent configurations.
	parent_configurations: u64, }

impl BayesianCategoricalReferenceSet {
	/// Reconstruct observations and derive their mixed-radix contract.
	///
	/// # Errors
	///
	/// Returns an error when a parent cardinality or derived configuration
	/// exceeds the supported categorical domain, or when the observations are
	/// structurally inconsistent.
	pub(crate) fn from_observations( parents: Vec<BayesianCategoricalSchema>, child: BayesianCategoricalSchema,
		reference_source_rows: Vec<usize>, parent_codes: Vec<i32>, child_codes: Vec<i32>, ) -> TrainingCompileResult<Self> {
		let mut parent_cardinalities = Vec::with_capacity(parents.len()); for parent in &parents {
			let cardinality = parent.dictionary.len().checked_add(1).ok_or_else(|| { return bayes_compile_error(
					TrainingCompileErrorKind::ArithmeticOverflow,
					"Bayesian parent cardinality overflowed usize",
				); })?; parent_cardinalities.push(i32::try_from(cardinality).map_err(|error| { return bayes_compile_error(
					TrainingCompileErrorKind::UnsupportedExtent,
					format!("Bayesian parent cardinality exceeds int32: {error}"),
				); })?); }
		let (parent_multipliers, parent_configurations) = mixed_radix_contract(&parent_cardinalities)?;
		let references = Self { parents, child, reference_rows: reference_source_rows.len(), reference_source_rows,
			parent_codes, child_codes, parent_cardinalities, parent_multipliers, parent_configurations, };
		validate_categorical_reference_set(&references)?; return Ok(references); }

	#[must_use]
	#[inline]
	pub fn parents(&self) -> &[BayesianCategoricalSchema] { return &self.parents; }

	#[must_use]
	#[inline]
	pub const fn child(&self) -> &BayesianCategoricalSchema { return &self.child; }

	#[must_use]
	#[inline]
	pub fn reference_source_rows(&self) -> &[usize] { return &self.reference_source_rows; }

	#[must_use]
	#[inline]
	pub const fn reference_rows(&self) -> usize { return self.reference_rows; }

	#[must_use]
	#[inline]
	pub fn parent_codes(&self) -> &[i32] { return &self.parent_codes; }

	#[must_use]
	#[inline]
	pub fn child_codes(&self) -> &[i32] { return &self.child_codes; }

	#[must_use]
	#[inline]
	pub fn parent_cardinalities(&self) -> &[i32] { return &self.parent_cardinalities; }

	#[must_use]
	#[inline]
	pub fn parent_multipliers(&self) -> &[i32] { return &self.parent_multipliers; }

	#[must_use]
	#[inline]
	pub const fn parent_configurations(&self) -> u64 { return self.parent_configurations; }

	#[must_use]
	#[inline]
	pub const fn child_classes(&self) -> usize { return self.child.dictionary.len(); }

	/// Append a compatible observation partition in source order.
	///
	/// # Errors
	///
	/// Returns an error when the schemas differ, row accounting overflows, an
	/// allocation cannot be reserved, or the combined observations are invalid.
	pub(crate) fn append(&mut self, current: Self) -> TrainingCompileResult<()> { if self.parents != current.parents
			|| self.child != current.child
			|| self.parent_cardinalities != current.parent_cardinalities
			|| self.parent_multipliers != current.parent_multipliers
			|| self.parent_configurations != current.parent_configurations
		{ return Err(bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
				"saved and current categorical Bayesian schemas or declaration order differ",
			)); }
		let rows = self .reference_rows .checked_add(current.reference_rows) .ok_or_else(|| { return bayes_compile_error(
					TrainingCompileErrorKind::ArithmeticOverflow,
					"combined categorical Bayesian reference row count overflowed usize",
				); })?; self.reference_source_rows .try_reserve_exact(current.reference_source_rows.len()) .map_err(|error| {
				return bayes_compile_error( TrainingCompileErrorKind::ArithmeticOverflow,
					format!("reserve categorical Bayesian source rows: {error}"),
				); })?; self.parent_codes .try_reserve_exact(current.parent_codes.len()) .map_err(|error| {
				return bayes_compile_error( TrainingCompileErrorKind::ArithmeticOverflow,
					format!("reserve categorical Bayesian parent observations: {error}"),
				); })?; self.child_codes .try_reserve_exact(current.child_codes.len()) .map_err(|error| {
				return bayes_compile_error( TrainingCompileErrorKind::ArithmeticOverflow,
					format!("reserve categorical Bayesian child observations: {error}"),
				); })?; self.reference_source_rows .extend(current.reference_source_rows);
		self.parent_codes.extend(current.parent_codes); self.child_codes.extend(current.child_codes);
		self.reference_rows = rows; return validate_categorical_reference_set(self); } }

/// Prepare one or more observed categorical target conditionals without
/// performing any host-side model calculation.
///
/// Declarations and data targets have the same order. Every child is a target,
/// every parent is an inference feature, and all nodes are observed
/// dictionary-categorical vectors. A target-as-parent edge is rejected by that
/// role boundary; inference for dependent unobserved children is a separate
/// instrument rather than an implied ancestral or marginal calculation.
///
/// # Errors
///
/// Returns an error when the declarations do not form the supported observed
/// categorical network, the prepared rows are empty or incompatible, or the
/// derived categorical extents exceed supported bounds.
#[inline]
pub fn prepare_categorical_bayesian_reference_sets( dataset: &PreparedDataset, dependencies: &[BayesianDependency],
) -> TrainingCompileResult<Vec<BayesianCategoricalReferenceSet>> {
	let schema = resolve_bayesian_schema(dataset, dependencies).map_err(|error| { return bayes_compile_error(
			TrainingCompileErrorKind::InvalidNetwork,
			format!("resolve Bayesian DAG: {error}"),
		); })?; if dependencies.is_empty() { return Err(bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
			"observed categorical Bayesian preparation requires at least one `.bayes(child, parents)` declaration",
		)); }
	if schema.nodes().iter().any(BayesianNodeSchema::is_latent) { return Err(bayes_compile_error(
			TrainingCompileErrorKind::InvalidNetwork,
			"latent Bayesian nodes need an explicit state space and are not part of the observed categorical slice",
		)); }
	if dataset.train().is_empty() { return Err(bayes_compile_error( TrainingCompileErrorKind::EmptyDataset,
			"categorical Bayesian preparation requires at least one training row",
		)); }

	let mut target_sources = Vec::with_capacity(dependencies.len()); for dependency in dependencies {
		if dependency.parents.is_empty() { return Err(bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
				"each observed categorical Bayesian conditional requires at least one observed parent",
			)); }
		let child = find_vector(dataset, dependency.child())?; if child.role() != VectorRole::Target {
			return Err(bayes_vector_error( child,
				"each declared Bayesian child must be a declared data target",
			)); }
		target_sources.push(child.source_index()); }
	if dataset.target_source_indices() != target_sources { return Err(bayes_compile_error(
			TrainingCompileErrorKind::InvalidTargetMatrix,
			"observed categorical Bayesian targets must equal the declared children in repeated-call order",
		)); }

	return dependencies .iter() .map(|dependency| {
			return prepare_categorical_bayesian_reference_set_for_dependency(dataset, dependency); }) .collect(); }

/// Prepare the observations and derived contract for one resolved dependency.
fn prepare_categorical_bayesian_reference_set_for_dependency( dataset: &PreparedDataset,
	dependency: &BayesianDependency, ) -> TrainingCompileResult<BayesianCategoricalReferenceSet> {
	let child = find_vector(dataset, dependency.child())?;
	let child_schema = categorical_schema(child, "child")?;
	let mut parents = Vec::with_capacity(dependency.parents().len());
	let mut parent_vectors = Vec::with_capacity(dependency.parents().len()); for parent_name in dependency.parents() {
		let parent = find_vector(dataset, parent_name)?; if parent.role() != VectorRole::Feature {
			return Err(bayes_vector_error( parent,
				"the observed categorical Bayesian slice requires every parent to be an inference feature",
			)); }
		parents.push(categorical_schema(parent, "parent")?);
		parent_vectors.push(parent); }

	let mut parent_cardinalities = Vec::with_capacity(parents.len()); for parent in &parents {
		let cardinality = parent.inference_cardinality().ok_or_else(|| { return bayes_compile_error(
				TrainingCompileErrorKind::ArithmeticOverflow, format!(
					"Bayesian parent {:?} cardinality overflowed usize",
					String::from_utf8_lossy(parent.name()) ), ); })?;
		parent_cardinalities.push(i32::try_from(cardinality).map_err(|error| { return bayes_compile_error(
				TrainingCompileErrorKind::UnsupportedExtent, format!(
					"Bayesian parent {:?} cardinality exceeds int32: {error}",
					String::from_utf8_lossy(parent.name()) ), ); })?); }
	let (parent_multipliers, parent_configurations) = mixed_radix_contract(&parent_cardinalities)?;
	let child_classes = u64::try_from(child_schema.dictionary.len()).map_err(|error| { return bayes_compile_error(
			TrainingCompileErrorKind::UnsupportedExtent,
			format!("Bayesian child class count exceeds u64: {error}"),
		); })?; if child_classes == 0
		|| parent_configurations .checked_mul(child_classes) .is_none_or(|bins| {
				return bins > i32::MAX as u64 || bins > u64::from(u32::MAX); })
	{ return Err(bayes_compile_error( TrainingCompileErrorKind::UnsupportedExtent,
			"categorical Bayesian joint state space is empty or exceeds the checked histogram domain",
		)); }

	let child_values = categorical_values(child)?; let mut parent_values = Vec::with_capacity(parent_vectors.len());
	for parent in &parent_vectors { parent_values.push(categorical_values(parent)?); }
	let parent_elements = dataset .train() .len() .checked_mul(parents.len()) .ok_or_else(|| { return bayes_compile_error(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"categorical Bayesian reference parent shape overflowed usize",
			); })?; let mut parent_codes = Vec::with_capacity(parent_elements);
	let mut child_codes = Vec::with_capacity(dataset.train().len()); for (&position, &source_row) in dataset .train()
		.retained_positions() .iter() .zip(dataset.train().source_rows())
	{ for (parent_index, (parent, values)) in parent_vectors.iter().zip(&parent_values).enumerate() {
			parent_codes.push(require_known_code( parent, values, position, source_row, parents[parent_index].dictionary.len(),
			)?); }
		child_codes.push(require_known_code( child, child_values, position, source_row, child_schema.dictionary.len(), )?); }
	let references = BayesianCategoricalReferenceSet { parents, child: child_schema,
		reference_source_rows: dataset.train().source_rows().to_vec(), reference_rows: dataset.train().len(), parent_codes,
		child_codes, parent_cardinalities, parent_multipliers, parent_configurations, };
	validate_categorical_reference_set(&references)?; return Ok(references); }

/// Validate categorical observation shapes, identities, codes, and radix metadata.
pub(crate) fn validate_categorical_reference_set( references: &BayesianCategoricalReferenceSet,
) -> TrainingCompileResult<()> { if references.parents.is_empty() || references.reference_rows == 0
		|| references.child.dictionary.is_empty()
		|| references.parent_cardinalities.len() != references.parents.len()
		|| references.parent_multipliers.len() != references.parents.len()
		|| references.child_codes.len() != references.reference_rows
		|| references.reference_source_rows.len() != references.reference_rows
		|| references.parent_codes.len() != references.reference_rows.saturating_mul(references.parents.len())
	{ return Err(bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
			"categorical Bayesian reference shapes are inconsistent",
		)); }
	let mut names = BTreeSet::new(); let mut sources = BTreeSet::new(); for vector in references .parents .iter()
		.chain(once(&references.child))
	{ if vector.name.is_empty() || vector.dictionary.is_empty() || !names.insert(vector.name.as_slice())
			|| !sources.insert(vector.source_index)
		{ return Err(bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
				"categorical Bayesian node names, source identities, and dictionaries must be nonempty and unique",
			)); }
		let mut labels = BTreeSet::new(); if vector .dictionary .iter() .any(|label| {
				return !labels.insert(label.as_slice()); })
		{ return Err(bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
				"categorical Bayesian dictionaries must contain unique labels",
			)); } }
	for (parent_index, parent) in references.parents.iter().enumerate() {
		let cardinality = usize::try_from(references.parent_cardinalities[parent_index]).map_err(|error| {
			return bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
				format!("categorical Bayesian parent cardinality is negative: {error}"),
			); })?; if cardinality != parent.dictionary.len() + 1 { return Err(bayes_compile_error(
				TrainingCompileErrorKind::InvalidNetwork,
				"categorical Bayesian parent cardinality does not reserve exactly one unseen route",
			)); } }
	let (multipliers, configurations) = mixed_radix_contract(&references.parent_cardinalities)?;
	if multipliers != references.parent_multipliers || configurations != references.parent_configurations {
		return Err(bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
			"categorical Bayesian mixed-radix metadata is inconsistent",
		)); }
	for (index, code) in references.parent_codes.iter().copied().enumerate() {
		let parent = index % references.parents.len(); if usize::try_from(code).map_or(true, |parsed_code| {
			return parsed_code >= references.parents[parent].dictionary.len(); }) { return Err(bayes_compile_error(
				TrainingCompileErrorKind::InvalidNetwork,
				format!("categorical Bayesian parent observation {index} has invalid code {code}"),
			)); } }
	if references .child_codes .iter() .copied() .any(|code| { return usize::try_from(code).map_or(true, |parsed_code| {
				return parsed_code >= references.child.dictionary.len(); }); })
	{ return Err(bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
			"categorical Bayesian child observations contain an invalid class code",
		)); }
	return Ok(()); }

/// Resolve one prepared vector by its exact name bytes.
fn find_vector<'a>(dataset: &'a PreparedDataset, name: &[u8]) -> TrainingCompileResult<&'a PreparedVector> {
	return dataset .vectors() .iter() .find(|vector| { return vector.name() == name; }) .ok_or_else(|| {
			return bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork, format!(
					"Bayesian node {:?} is absent from prepared data",
					String::from_utf8_lossy(name) ), ); }); }

/// Extract the exact dictionary schema required for a Bayesian vector role.
fn categorical_schema(vector: &PreparedVector, role: &str) -> TrainingCompileResult<BayesianCategoricalSchema> {
	if vector.semantic_type() != SemanticType::Categorical || vector.encoding() != VectorEncoding::DictionaryI32 {
		return Err(bayes_vector_error( vector,
			format!("the executable Bayesian {role} must be dictionary-encoded categorical data"),
		)); }
	let VectorMetadata::Categorical { dictionary } = vector.metadata().clone() else { return Err(bayes_vector_error(
			vector,
			format!("the executable Bayesian {role} must retain a categorical dictionary"),
		)); }; if dictionary.is_empty() { return Err(bayes_vector_error( vector,
			"categorical dictionary is empty",
		)); }
	return Ok(BayesianCategoricalSchema { source_index: vector.source_index(), name: vector.name().to_vec(), dictionary,
	}); }

/// Borrow categorical calculation codes from prepared integer storage.
fn categorical_values(vector: &PreparedVector) -> TrainingCompileResult<&[Option<i32>]> {
	return vector.values().i32_values().ok_or_else(|| {
		return bayes_vector_error(vector, "categorical values are not stored as int32 codes");
	}); }

/// Resolve one complete categorical code and verify its dictionary bound.
fn require_known_code( vector: &PreparedVector, values: &[Option<i32>], position: usize, source_row: usize,
	classes: usize, ) -> TrainingCompileResult<i32> { let code = values .get(position) .ok_or_else(|| {
			return bayes_vector_error( vector,
				format!("source row {source_row} is absent from prepared storage"),
			); })? .ok_or_else(|| { return bayes_vector_error( vector, format!(
					"source row {source_row} is missing; the observed categorical slice requires complete rows"
				), ); })?; if usize::try_from(code).map_or(true, |parsed_code| { return parsed_code >= classes; }) {
		return Err(bayes_vector_error( vector,
			format!("source row {source_row} has out-of-dictionary code {code}"),
		)); }
	return Ok(code); }

/// Derive right-to-left multipliers and the total parent configuration count.
fn mixed_radix_contract(cardinalities: &[i32]) -> TrainingCompileResult<(Vec<i32>, u64)> {
	let mut configurations = 1u64; let mut multipliers = vec![0i32; cardinalities.len()];
	for (index, signed_cardinality) in cardinalities.iter().copied().enumerate().rev() {
		let cardinality = u64::try_from(signed_cardinality).map_err(|error| { return bayes_compile_error(
				TrainingCompileErrorKind::InvalidNetwork,
				format!("categorical Bayesian parent cardinality must be positive: {error}"),
			); })?; if cardinality == 0 { return Err(bayes_compile_error( TrainingCompileErrorKind::InvalidNetwork,
				"categorical Bayesian parent cardinality must be positive",
			)); }
		multipliers[index] = i32::try_from(configurations).map_err(|error| { return bayes_compile_error(
				TrainingCompileErrorKind::UnsupportedExtent,
				format!("categorical Bayesian parent multiplier exceeds int32: {error}"),
			); })?; configurations = configurations.checked_mul(cardinality).ok_or_else(|| { return bayes_compile_error(
				TrainingCompileErrorKind::ArithmeticOverflow,
				"categorical Bayesian parent configuration count overflowed u64",
			); })?; }
	if configurations > 0x7fff_ffff { return Err(bayes_compile_error( TrainingCompileErrorKind::UnsupportedExtent,
			"categorical Bayesian parent configuration count exceeds int32",
		)); }
	return Ok((multipliers, configurations)); }

/// Construct a target-matrix error anchored to one prepared vector.
fn bayes_vector_error(vector: &PreparedVector, detail: impl Into<String>) -> TrainingCompileError {
	return bayes_compile_error( TrainingCompileErrorKind::InvalidTargetMatrix, format!(
			"Bayesian vector {:?} at source index {}: {}",
			String::from_utf8_lossy(vector.name()), vector.source_index(), detail.into() ), ); }

/// Construct a typed Bayesian training compilation error.
fn bayes_compile_error(kind: TrainingCompileErrorKind, detail: impl Into<String>) -> TrainingCompileError {
	return TrainingCompileError::new(kind, detail); }

/// Typed reason a Bayesian schema could not be resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BayesianSchemaErrorKind { EmptyName, DuplicateDatasetName, DuplicateChild, DuplicateParent, SelfDependency,
	Cycle, }

impl fmt::Display for BayesianSchemaErrorKind {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		return f.write_str(match *self {
			Self::EmptyName => "empty name",
			Self::DuplicateDatasetName => "duplicate dataset name",
			Self::DuplicateChild => "duplicate child",
			Self::DuplicateParent => "duplicate parent",
			Self::SelfDependency => "self dependency",
			Self::Cycle => "cycle",
		}); } }

/// One typed segment in an error's structural path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum BayesianSchemaPathSegment { Dataset, Vectors, Vector(usize), Name, Declarations, Declaration(usize), Child,
	Parents, Parent(usize), Graph, ExecutionOrder, }

/// A structural Bayesian resolution error with a machine-readable hierarchy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesianSchemaError {
	/// Machine-readable failure category.
	kind: BayesianSchemaErrorKind,
	/// Structural location of the invalid declaration or dataset element.
	path: Vec<BayesianSchemaPathSegment>,
	/// Human-readable failure detail.
	detail: String, }

impl BayesianSchemaError {
	/// Construct a structural schema error from its typed components.
	fn new( kind: BayesianSchemaErrorKind, path: impl Into<Vec<BayesianSchemaPathSegment>>, detail: impl Into<String>,
	) -> Self { return Self { kind, path: path.into(), detail: detail.into(), }; }

	#[must_use]
	#[inline]
	pub const fn kind(&self) -> BayesianSchemaErrorKind { return self.kind; }

	#[must_use]
	#[inline]
	pub fn path(&self) -> &[BayesianSchemaPathSegment] { return &self.path; }

	#[must_use]
	#[inline]
	pub fn detail(&self) -> &str { return &self.detail; } }

impl fmt::Display for BayesianSchemaError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		return write!( f,
			"{} at {}: {}",
			self.kind, DisplayPath(&self.path), self.detail ); } }

impl Error for BayesianSchemaError {}

/// Dot-and-index rendering of a typed Bayesian schema path.
struct DisplayPath<'a>(
	/// Ordered structural path segments.
	&'a [BayesianSchemaPathSegment],
);

impl fmt::Display for DisplayPath<'_> {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for segment in self.0 { match *segment {
				BayesianSchemaPathSegment::Dataset => f.write_str("dataset")?,
				BayesianSchemaPathSegment::Vectors => f.write_str(".vectors")?,
				BayesianSchemaPathSegment::Vector(vector) => write!(f, "[{vector}]")?,
				BayesianSchemaPathSegment::Name => f.write_str(".name")?,
				BayesianSchemaPathSegment::Declarations => f.write_str("declarations")?,
				BayesianSchemaPathSegment::Declaration(declaration) => write!(f, "[{declaration}]")?,
				BayesianSchemaPathSegment::Child => f.write_str(".child")?,
				BayesianSchemaPathSegment::Parents => f.write_str(".parents")?,
				BayesianSchemaPathSegment::Parent(parent) => write!(f, "[{parent}]")?,
				BayesianSchemaPathSegment::Graph => f.write_str("graph")?,
				BayesianSchemaPathSegment::ExecutionOrder => f.write_str(".execution-order")?,
			} }
		return Ok(()); } }

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
#[inline]
pub fn resolve_bayesian_schema( dataset: &PreparedDataset, dependencies: &[BayesianDependency],
) -> BayesianSchemaResult<ResolvedBayesianSchema> { let mut observed = BTreeMap::new();
	for (index, vector) in dataset.vectors().iter().enumerate() { if observed
			.insert(vector.name().to_vec(), vector.schema()) .is_some()
		{ return Err(BayesianSchemaError::new( BayesianSchemaErrorKind::DuplicateDatasetName, [
					BayesianSchemaPathSegment::Dataset, BayesianSchemaPathSegment::Vectors, BayesianSchemaPathSegment::Vector(index),
					BayesianSchemaPathSegment::Name, ], format!(
					"prepared vector name {:?} appears more than once",
					String::from_utf8_lossy(vector.name()) ), )); } }
	validate_dependencies(dependencies)?;

	let mut sources = observed .into_iter() .map(|(name, schema)| { return (name, BayesianNodeSource::Observed(schema)); })
		.collect::<BTreeMap<_, _>>(); for dependency in dependencies { match sources.entry(dependency.child.clone()) {
			Entry::Vacant(entry) => { entry.insert(if dependency.parents.is_empty() { BayesianNodeSource::LatentRoot } else {
					BayesianNodeSource::LatentConditional }); }
			Entry::Occupied(mut entry) => {
				if !dependency.parents.is_empty() && matches!(entry.get(), BayesianNodeSource::LatentRoot) {
					entry.insert(BayesianNodeSource::LatentConditional); } } }
		for parent in &dependency.parents { sources .entry(parent.clone()) .or_insert(BayesianNodeSource::LatentRoot); } }

	let nodes = sources .into_iter() .enumerate() .map(|(index, (name, source))| { return BayesianNodeSchema {
				id: BayesianNodeId(index), name, source, }; }) .collect::<Vec<_>>(); let ids = nodes .iter() .map(|node| {
			return (node.name.clone(), node.id); }) .collect::<BTreeMap<_, _>>(); let declarations = dependencies .iter()
		.enumerate() .map(|(declaration_index, dependency)| { return ResolvedBayesianDependency { declaration_index,
				child: ids[&dependency.child], parents: dependency .parents .iter() .map(|parent| { return ids[parent]; })
					.collect(), }; }) .collect::<Vec<_>>();
	let execution_order = deterministic_topological_order(&nodes, &declarations)?;

	return Ok(ResolvedBayesianSchema { nodes, declarations, execution_order, }); }

/// Validate declaration names and repeated edges before graph construction.
fn validate_dependencies(dependencies: &[BayesianDependency]) -> BayesianSchemaResult<()> {
	let mut children = BTreeMap::<&[u8], usize>::new();
	for (declaration_index, dependency) in dependencies.iter().enumerate() { let declaration_path = || { return vec![
				BayesianSchemaPathSegment::Declarations, BayesianSchemaPathSegment::Declaration(declaration_index), ]; };
		if dependency.child.is_empty() { let mut path = declaration_path(); path.push(BayesianSchemaPathSegment::Child);
			return Err(BayesianSchemaError::new( BayesianSchemaErrorKind::EmptyName, path,
				"Bayesian child name is empty",
			)); }
		if let Some(first) = children.insert(&dependency.child, declaration_index) { let mut path = declaration_path();
			path.push(BayesianSchemaPathSegment::Child); return Err(BayesianSchemaError::new(
				BayesianSchemaErrorKind::DuplicateChild, path, format!(
					"Bayesian child {:?} was already declared at declarations[{first}]",
					String::from_utf8_lossy(&dependency.child) ), )); }

		let mut parents = BTreeMap::<&[u8], usize>::new();
		for (parent_index, parent) in dependency.parents.iter().enumerate() { let parent_path = || {
				let mut path = declaration_path(); path.extend([ BayesianSchemaPathSegment::Parents,
					BayesianSchemaPathSegment::Parent(parent_index), ]); return path; }; if parent.is_empty() {
				return Err(BayesianSchemaError::new( BayesianSchemaErrorKind::EmptyName, parent_path(),
					"Bayesian parent name is empty",
				)); }
			if parent == &dependency.child { return Err(BayesianSchemaError::new( BayesianSchemaErrorKind::SelfDependency,
					parent_path(), format!(
						"Bayesian node {:?} cannot name itself as a parent",
						String::from_utf8_lossy(parent) ), )); }
			if let Some(first) = parents.insert(parent, parent_index) { return Err(BayesianSchemaError::new(
					BayesianSchemaErrorKind::DuplicateParent, parent_path(), format!(
						"Bayesian parent {:?} was already declared at parents[{first}]",
						String::from_utf8_lossy(parent) ), )); } } }
	return Ok(()); }

/// Produce the stable name-ordered topological traversal or report a cycle.
fn deterministic_topological_order( nodes: &[BayesianNodeSchema], declarations: &[ResolvedBayesianDependency],
) -> BayesianSchemaResult<Vec<BayesianNodeId>> { let mut outgoing = vec![Vec::new(); nodes.len()];
	let mut indegree = vec![0usize; nodes.len()]; for declaration in declarations { for parent in &declaration.parents {
			outgoing[parent.index()].push(declaration.child); indegree[declaration.child.index()] += 1; } }
	let mut ready = indegree .iter() .enumerate() .filter_map(|(index, degree)| {
			return (*degree == 0).then_some(BayesianNodeId(index)); }) .collect::<BTreeSet<_>>();
	let mut order = Vec::with_capacity(nodes.len()); while let Some(node) = ready.iter().next().copied() {
		ready.remove(&node); order.push(node); for child in &outgoing[node.index()] { indegree[child.index()] -= 1;
			if indegree[child.index()] == 0 { ready.insert(*child); } } }
	if order.len() != nodes.len() { let names = indegree .iter() .enumerate() .filter(|entry| { return *entry.1 != 0; })
			.map(|entry| { return String::from_utf8_lossy(nodes[entry.0].name()).into_owned(); }) .collect::<Vec<_>>();
		return Err(BayesianSchemaError::new( BayesianSchemaErrorKind::Cycle, [ BayesianSchemaPathSegment::Graph,
				BayesianSchemaPathSegment::ExecutionOrder, ],
			format!("Bayesian declarations contain a dependency cycle among {names:?}"),
		)); }
	return Ok(order); }
