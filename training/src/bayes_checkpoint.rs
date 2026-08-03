use alloc::collections::{BTreeMap, BTreeSet};
use core::{fmt::Write as _, iter::once, str::from_utf8};
use std::{io::Write as _, path::Path};

use recipe_ogdl::{Graph, NodeId};

use crate::{
	CheckpointError, CheckpointResult,
	bayes::{
		BayesianCategoricalReferenceSet, BayesianCategoricalSchema, CATEGORICAL_BAYES_SMOOTHING,
		validate_categorical_reference_set,
	},
	checkpoint::atomic_save,
};

/// Singular-conditional Bayesian semantic model format version.
const BAYES_MODEL_FORMAT_VERSION_ONE: u32 = 1;
/// Repeated-conditional Bayesian semantic model format version.
const BAYES_MODEL_FORMAT_VERSION_TWO: u32 = 2;
/// Canonical root tag for Bayesian semantic model graphs.
const ROOT: &str = "recipe-bayes-model";

/// Finite decode and allocation limits for an observed categorical Bayesian
/// semantic model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BayesModelDecodeLimits {
	pub source_bytes: usize,
	pub nodes: usize,
	pub conditionals: usize,
	pub parents: usize,
	pub labels: usize,
	pub reference_rows: usize,
	pub total_payload_bytes: usize,
}

impl Default for BayesModelDecodeLimits {
	#[inline]
	fn default() -> Self {
		return Self {
			source_bytes: 1 << 30,
			nodes: 4_000_000,
			conditionals: 0x0001_0000,
			parents: 0x0001_0000,
			labels: 1_000_000,
			reference_rows: 100_000_000,
			total_payload_bytes: 1 << 30,
		};
	}
}

/// Complete semantic model for one or more observed categorical Bayesian
/// target conditionals.
///
/// No fitted count table or opaque native state is stored. The exact observed
/// codes and label dictionaries are sufficient for Recipe to reconstruct the
/// same Laplace posterior on any supported native target.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BayesModelArtifact {
	/// Semantic model format version.
	format_version: u32,
	/// Exact smoothing coefficient bits.
	smoothing_bits: u32,
	/// Ordered observed categorical conditionals.
	conditionals: Vec<BayesianCategoricalReferenceSet>,
}

impl BayesModelArtifact {
	/// Construct a version-one artifact containing one categorical conditional.
	///
	/// # Errors
	///
	/// Returns an error when the supplied reference set violates the semantic
	/// artifact contract.
	#[inline]
	pub fn new(references: BayesianCategoricalReferenceSet) -> CheckpointResult<Self> {
		let artifact = Self {
			format_version: BAYES_MODEL_FORMAT_VERSION_ONE,
			smoothing_bits: CATEGORICAL_BAYES_SMOOTHING.to_bits(),
			conditionals: vec![references],
		};
		validate_artifact(&artifact)?;
		return Ok(artifact);
	}

	/// Construct the repeated-call observed categorical instrument. A singular
	/// model retains the version-one image; two or more conditionals use the
	/// version-two repeated structure.
	///
	/// # Errors
	///
	/// Returns an error when the ordered conditionals do not form a valid
	/// singular or repeated semantic artifact.
	#[inline]
	pub fn from_conditionals(conditionals: Vec<BayesianCategoricalReferenceSet>) -> CheckpointResult<Self> {
		let format_version = if conditionals.len() == 1 {
			BAYES_MODEL_FORMAT_VERSION_ONE
		} else {
			BAYES_MODEL_FORMAT_VERSION_TWO
		};
		let artifact = Self {
			format_version,
			smoothing_bits: CATEGORICAL_BAYES_SMOOTHING.to_bits(),
			conditionals,
		};
		validate_artifact(&artifact)?;
		return Ok(artifact);
	}

	#[must_use]
	#[inline]
	pub const fn format_version(&self) -> u32 {
		return self.format_version;
	}

	#[must_use]
	#[inline]
	pub const fn smoothing(&self) -> f32 {
		return f32::from_bits(self.smoothing_bits);
	}

	#[must_use]
	#[inline]
	pub fn conditionals(&self) -> &[BayesianCategoricalReferenceSet] {
		return &self.conditionals;
	}

	/// Continue with another complete observed partition. Repeated rows remain
	/// repeated evidence; saved observations remain before current observations.
	///
	/// # Errors
	///
	/// Returns an error when either artifact is invalid, their contracts differ,
	/// or their observation partitions cannot be combined.
	#[inline]
	pub fn continue_with(mut self, current: Self) -> CheckpointResult<Self> {
		validate_artifact(&self)?;
		validate_artifact(&current)?;
		if self.format_version != current.format_version || self.smoothing_bits != current.smoothing_bits {
			return Err(CheckpointError::manifest(
				"saved and current categorical Bayesian model contracts differ",
			));
		}
		if self.conditionals.len() != current.conditionals.len() {
			return Err(CheckpointError::manifest(
				"saved and current categorical Bayesian conditional counts differ",
			));
		}
		for (saved, incoming) in self.conditionals.iter_mut().zip(current.conditionals) {
			saved.append(incoming)
				.map_err(|error| {
					return CheckpointError::manifest(error.to_string());
				})?;
		}
		validate_artifact(&self)?;
		return Ok(self);
	}

	/// Encode this artifact as canonical textual OGDL bytes.
	///
	/// # Errors
	///
	/// Returns an error when the artifact is invalid or its OGDL graph cannot
	/// be constructed.
	#[inline]
	pub fn encode(&self) -> CheckpointResult<Vec<u8>> {
		validate_artifact(self)?;
		let mut graph = Graph::new();
		let root = graph.append_root(ROOT).map_err(|error| {
			return CheckpointError::manifest(format!("encode Bayesian root: {error}"));
		})?;
		field(&mut graph, root, "format-version", &self.format_version)?;
		field(&mut graph, root, "smoothing", "laplace-one")?;
		if self.format_version == BAYES_MODEL_FORMAT_VERSION_ONE {
			encode_reference(&mut graph, root, &self.conditionals[0])?;
		} else {
			let conditionals = child(&mut graph, root, "conditionals")?;
			for references in &self.conditionals {
				let conditional = child(&mut graph, conditionals, "conditional")?;
				encode_reference(&mut graph, conditional, references)?;
			}
		}
		return Ok(graph.to_canonical_string().into_bytes());
	}

	/// Decode one canonical Bayesian semantic model under finite limits.
	///
	/// # Errors
	///
	/// Returns an error when the source is malformed, noncanonical, invalid, or
	/// exceeds a configured decode limit.
	#[inline]
	pub fn decode(source: &[u8], limits: BayesModelDecodeLimits) -> CheckpointResult<Self> {
		return decode_bayes_model(source, limits);
	}

	/// Atomically save this semantic model to an `.ogdl` path.
	///
	/// # Errors
	///
	/// Returns an error for a non-OGDL path, invalid artifact, oversized image,
	/// encoding failure, or atomic file-write failure.
	#[inline]
	pub fn save(&self, path: impl AsRef<Path>) -> CheckpointResult<()> {
		let target = path.as_ref();
		if target
			.extension()
			.and_then(|extension| {
				return extension.to_str();
			}) != Some("ogdl")
		{
			return Err(CheckpointError::invalid_target(
				target,
				"Bayesian semantic model path must end in .ogdl",
			));
		}
		let encoded = self.encode()?;
		let bytes = u64::try_from(encoded.len()).map_err(|error| {
			return CheckpointError::invalid_target(target, format!("Bayesian model size exceeds u64: {error}"));
		})?;
		return atomic_save(target, bytes, |file| {
			return file.write_all(&encoded);
		});
	}
}

/// Decode canonical Bayesian semantic-model bytes under finite limits.
///
/// # Errors
///
/// Returns an error when the source is malformed, noncanonical, semantically
/// invalid, or exceeds a configured decode limit.
#[inline]
pub fn decode_bayes_model(source: &[u8], limits: BayesModelDecodeLimits) -> CheckpointResult<BayesModelArtifact> {
	if source.len() > limits.source_bytes {
		return Err(CheckpointError::manifest(format!(
			"Bayesian model source has {} bytes, limit is {}",
			source.len(),
			limits.source_bytes
		)));
	}
	let text = from_utf8(source).map_err(|error| {
		return CheckpointError::manifest(format!("Bayesian model is not UTF-8: {error}"));
	})?;
	let graph = Graph::parse(text).map_err(|error| {
		return CheckpointError::manifest(format!("invalid Bayesian OGDL: {error}"));
	})?;
	if graph.len() > limits.nodes {
		return Err(CheckpointError::manifest(format!(
			"Bayesian model has {} nodes, limit is {}",
			graph.len(),
			limits.nodes
		)));
	}
	return Decoder {
		graph,
		limits,
		payload_bytes: 0,
		labels: 0,
		parents: 0,
		reference_rows: 0,
	}
	.decode(source);
}

/// Validate versioning, smoothing, schemas, partitions, and child-parent roles.
fn validate_artifact(artifact: &BayesModelArtifact) -> CheckpointResult<()> {
	if !matches!(
		artifact.format_version,
		BAYES_MODEL_FORMAT_VERSION_ONE | BAYES_MODEL_FORMAT_VERSION_TWO
	) {
		return Err(CheckpointError::manifest(format!(
			"Bayesian model format version {} is unsupported",
			artifact.format_version
		)));
	}
	if artifact.conditionals.is_empty()
		|| (artifact.format_version == BAYES_MODEL_FORMAT_VERSION_ONE && artifact.conditionals.len() != 1)
		|| (artifact.format_version == BAYES_MODEL_FORMAT_VERSION_TWO && artifact.conditionals.len() < 2)
	{
		return Err(CheckpointError::manifest(
			"Bayesian model version and conditional count are inconsistent",
		));
	}
	if artifact.smoothing_bits != CATEGORICAL_BAYES_SMOOTHING.to_bits() {
		return Err(CheckpointError::manifest(
			"categorical Bayesian model must use Recipe's Laplace-one smoothing",
		));
	}
	for references in &artifact.conditionals {
		validate_categorical_reference_set(references)
			.map_err(|error| {
				return CheckpointError::manifest(error.to_string());
			})?;
	}
	let first = &artifact.conditionals[0];
	let mut children_by_name = BTreeSet::new();
	let mut children_by_source = BTreeSet::new();
	let mut schemas_by_name = BTreeMap::new();
	let mut schemas_by_source = BTreeMap::new();
	for references in &artifact.conditionals {
		if references.reference_rows() != first.reference_rows()
			|| references.reference_source_rows() != first.reference_source_rows()
		{
			return Err(CheckpointError::manifest(
				"Bayesian conditionals do not retain the same ordered reference partition",
			));
		}
		if !children_by_name.insert(references.child().name())
			|| !children_by_source.insert(references.child().source_index())
		{
			return Err(CheckpointError::manifest(
				"Bayesian conditional children must have unique names and source identities",
			));
		}
		for schema in references
			.parents()
			.iter()
			.chain(once(references.child()))
		{
			if let Some(existing) = schemas_by_name.insert(schema.name(), schema)
				&& existing != schema
			{
				return Err(CheckpointError::manifest(
					"Bayesian repeated node names have inconsistent saved schemas",
				));
			}
			if let Some(existing) = schemas_by_source.insert(schema.source_index(), schema)
				&& existing != schema
			{
				return Err(CheckpointError::manifest(
					"Bayesian repeated source identities have inconsistent saved schemas",
				));
			}
		}
	}
	if artifact.conditionals.iter().any(|references| {
		return references.parents().iter().any(|parent| {
			return children_by_name.contains(parent.name())
				|| children_by_source.contains(&parent.source_index());
		});
	}) {
		return Err(CheckpointError::manifest(
			"the observed multi-output Bayesian instrument does not accept a target child as another conditional's parent",
		));
	}
	return Ok(());
}

/// Append one categorical conditional to an OGDL graph.
///
/// # Errors
///
/// Returns an error when a graph node cannot be appended or a source row cannot
/// be represented by the checkpoint format.
fn encode_reference(
	graph: &mut Graph,
	root: NodeId,
	references: &BayesianCategoricalReferenceSet,
) -> CheckpointResult<()> {
	field(graph, root, "reference-rows", &references.reference_rows())?;
	let source_rows = references.reference_source_rows();
	let mut encoded_source_rows = String::with_capacity(2 + source_rows.len().saturating_mul(16));
	encoded_source_rows.push_str("0x");
	for value in source_rows {
		let encoded_value = u64::try_from(*value).map_err(|error| {
			return CheckpointError::manifest(format!("Bayesian source row exceeds u64: {error}"));
		})?;
		write!(encoded_source_rows, "{encoded_value:016x}").map_err(|error| {
			return CheckpointError::manifest(format!("encode Bayesian source row: {error}"));
		})?;
	}
	field(
		graph,
		root,
		"reference-source-rows",
		&encoded_source_rows,
	)?;
	let parents = child(graph, root, "parents")?;
	for parent in references.parents() {
		encode_schema(graph, parents, "parent", parent)?;
	}
	encode_schema(graph, root, "child", references.child())?;
	field(
		graph,
		root,
		"reference-parent-codes",
		&encode_i32_hex(references.parent_codes())?,
	)?;
	field(
		graph,
		root,
		"reference-child-codes",
		&encode_i32_hex(references.child_codes())?,
	)?;
	return Ok(());
}

/// Append one categorical schema beneath `parent`.
///
/// # Errors
///
/// Returns an error when an OGDL node cannot be appended.
fn encode_schema(
	graph: &mut Graph,
	parent: NodeId,
	tag: &str,
	schema: &BayesianCategoricalSchema,
) -> CheckpointResult<()> {
	let node = child(graph, parent, tag)?;
	field(graph, node, "source-index", &schema.source_index())?;
	field(graph, node, "name-bytes", &encode_bytes(schema.name())?)?;
	let labels = child(graph, node, "labels")?;
	for label in schema.dictionary() {
		field(graph, labels, "value-bytes", &encode_bytes(label)?)?;
	}
	return Ok(());
}

/// Append one child node beneath `parent`.
///
/// # Errors
///
/// Returns an error when the graph rejects the new child.
fn child(graph: &mut Graph, parent: NodeId, text: impl Into<String>) -> CheckpointResult<NodeId> {
	return graph.append_child(parent, text).map_err(|error| {
		return CheckpointError::manifest(format!("encode Bayesian OGDL node: {error}"));
	});
}

/// Append a named scalar field beneath `parent`.
///
/// # Errors
///
/// Returns an error when either field node cannot be appended.
fn field(
	graph: &mut Graph,
	parent: NodeId,
	name: impl Into<String>,
	value: &(impl ToString + ?Sized),
) -> CheckpointResult<()> {
	let field = child(graph, parent, name)?;
	child(graph, field, value.to_string())?;
	return Ok(());
}

/// Encode bytes as a lowercase hexadecimal scalar.
///
/// # Errors
///
/// Returns an error when the scalar cannot be formatted.
fn encode_bytes(bytes: &[u8]) -> CheckpointResult<String> {
	let mut output = String::with_capacity(2 + bytes.len().saturating_mul(2));
	output.push_str("0x");
	for byte in bytes {
		write!(output, "{byte:02x}").map_err(|error| {
			return CheckpointError::manifest(format!("encode Bayesian byte scalar: {error}"));
		})?;
	}
	return Ok(output);
}

/// Encode signed categorical codes by their two's-complement bit patterns.
///
/// # Errors
///
/// Returns an error when the scalar cannot be formatted.
fn encode_i32_hex(values: &[i32]) -> CheckpointResult<String> {
	let mut output = String::with_capacity(2 + values.len().saturating_mul(8));
	output.push_str("0x");
	for value in values {
		write!(output, "{:08x}", value.cast_unsigned()).map_err(|error| {
			return CheckpointError::manifest(format!("encode Bayesian categorical code: {error}"));
		})?;
	}
	return Ok(output);
}

/// Bounded state used while decoding a Bayesian checkpoint graph.
#[derive(Debug)]
struct Decoder {
	/// Parsed semantic checkpoint graph.
	graph: Graph,
	/// Caller-provided decode limits.
	limits: BayesModelDecodeLimits,
	/// Total decoded payload bytes consumed so far.
	payload_bytes: usize,
	/// Total decoded dictionary labels consumed so far.
	labels: usize,
	/// Total decoded parent schemas consumed so far.
	parents: usize,
	/// Total decoded reference rows consumed so far.
	reference_rows: usize,
}

impl Decoder {
	/// Decode and validate the complete Bayesian model artifact.
	///
	/// # Errors
	///
	/// Returns an error when the graph violates the Bayesian checkpoint schema,
	/// exceeds a decode limit, or is not canonically encoded.
	fn decode(mut self, source: &[u8]) -> CheckpointResult<BayesModelArtifact> {
		let &[root] = self.graph.roots() else {
			return Err(CheckpointError::manifest(format!(
				"Bayesian model requires exactly one root, found {}",
				self.graph.roots().len()
			)));
		};
		if self.node(root)?.text() != ROOT {
			return Err(CheckpointError::manifest(format!(
				"Bayesian model root must be {ROOT:?}"
			)));
		}
		let format_field = self.unique_field(root, "format-version")?;
		let format_value = self.scalar(format_field)?;
		let format_version = format_value.parse::<u32>().map_err(|error| {
			return CheckpointError::manifest(format!("Bayesian format-version is invalid: {error}"));
		})?;
		if format_version.to_string() != format_value {
			return Err(CheckpointError::manifest(
				"Bayesian format-version is not canonical decimal",
			));
		}
		let conditionals = match format_version {
			BAYES_MODEL_FORMAT_VERSION_ONE => {
				let fields = self.fields(
					root,
					&[
						"format-version",
						"smoothing",
						"reference-rows",
						"reference-source-rows",
						"parents",
						"child",
						"reference-parent-codes",
						"reference-child-codes",
					],
				)?;
				self.require_laplace_one(fields["smoothing"])?;
				vec![self.decode_reference_fields(&fields)?]
			}
			BAYES_MODEL_FORMAT_VERSION_TWO => {
				let fields = self.fields(root, &["format-version", "smoothing", "conditionals"])?;
				self.require_laplace_one(fields["smoothing"])?;
				let nodes = self.node(fields["conditionals"])?.children().to_vec();
				if nodes.len() < 2 || nodes.len() > self.limits.conditionals {
					return Err(CheckpointError::manifest(format!(
						"Bayesian conditional count is {}, expected 2..={}",
						nodes.len(),
						self.limits.conditionals
					)));
				}
				let mut conditionals = Vec::with_capacity(nodes.len());
				for node in nodes {
					if self.node(node)?.text() != "conditional" {
						return Err(CheckpointError::manifest(
							"Bayesian conditionals may contain only conditional entries",
						));
					}
					conditionals.push(self.decode_reference(node)?);
				}
				conditionals
			}
			_ => {
				return Err(CheckpointError::manifest(format!(
					"unsupported Bayesian model version {format_version}"
				)));
			}
		};
		let artifact = BayesModelArtifact {
			format_version,
			smoothing_bits: CATEGORICAL_BAYES_SMOOTHING.to_bits(),
			conditionals,
		};
		validate_artifact(&artifact)?;
		if artifact.encode()?.as_slice() != source {
			return Err(CheckpointError::manifest(
				"Bayesian model is valid but not in canonical textual OGDL form",
			));
		}
		return Ok(artifact);
	}

	/// Require the canonical categorical smoothing declaration.
	///
	/// # Errors
	///
	/// Returns an error when the field is not the `laplace-one` scalar.
	fn require_laplace_one(&self, field: NodeId) -> CheckpointResult<()> {
		if self.scalar(field)? != "laplace-one" {
			return Err(CheckpointError::manifest(
				"Bayesian smoothing must be the canonical laplace-one contract",
			));
		}
		return Ok(());
	}

	/// Decode one conditional node and its required fields.
	///
	/// # Errors
	///
	/// Returns an error when the node does not match the conditional schema or
	/// exceeds a configured decode limit.
	fn decode_reference(&mut self, node: NodeId) -> CheckpointResult<BayesianCategoricalReferenceSet> {
		let fields = self.fields(
			node,
			&[
				"reference-rows",
				"reference-source-rows",
				"parents",
				"child",
				"reference-parent-codes",
				"reference-child-codes",
			],
		)?;
		return self.decode_reference_fields(&fields);
	}

	/// Decode one categorical conditional from its indexed field nodes.
	///
	/// # Errors
	///
	/// Returns an error when fields are malformed, aggregate limits are exceeded,
	/// arithmetic overflows, or the decoded observations violate their schema.
	fn decode_reference_fields(
		&mut self,
		fields: &BTreeMap<String, NodeId>,
	) -> CheckpointResult<BayesianCategoricalReferenceSet> {
		let reference_rows = Self::parse_usize(self.scalar(fields["reference-rows"])?, "reference-rows")?;
		self.reference_rows = self
			.reference_rows
			.checked_add(reference_rows)
			.ok_or_else(|| {
				return CheckpointError::manifest("Bayesian aggregate reference row count overflowed usize");
			})?;
		if reference_rows == 0 || self.reference_rows > self.limits.reference_rows {
			return Err(CheckpointError::manifest(format!(
				"Bayesian aggregate reference row count is {}, limit is {}",
				self.reference_rows, self.limits.reference_rows
			)));
		}
		let parent_entries = self.node(fields["parents"])?.children().to_vec();
		self.parents = self
			.parents
			.checked_add(parent_entries.len())
			.ok_or_else(|| {
				return CheckpointError::manifest("Bayesian aggregate parent count overflowed usize");
			})?;
		if parent_entries.is_empty() || self.parents > self.limits.parents {
			return Err(CheckpointError::manifest(format!(
				"Bayesian aggregate parent count is {}, limit is {}",
				self.parents, self.limits.parents
			)));
		}
		let mut parents = Vec::with_capacity(parent_entries.len());
		for parent in parent_entries {
			if self.node(parent)?.text() != "parent" {
				return Err(CheckpointError::manifest(
					"Bayesian parents may contain only parent entries",
				));
			}
			parents.push(self.decode_schema(parent)?);
		}
		let child = self.decode_schema(fields["child"])?;
		let source_rows = self.scalar(fields["reference-source-rows"])?.to_owned();
		let reference_source_rows = self.decode_usize_hex(&source_rows, reference_rows, "reference-source-rows")?;
		let parent_elements = reference_rows.checked_mul(parents.len()).ok_or_else(|| {
			return CheckpointError::manifest("Bayesian reference parent element count overflowed usize");
		})?;
		let encoded_parent_codes = self.scalar(fields["reference-parent-codes"])?.to_owned();
		let parent_codes = self.decode_i32_hex(&encoded_parent_codes, parent_elements, "reference-parent-codes")?;
		let encoded_child_codes = self.scalar(fields["reference-child-codes"])?.to_owned();
		let child_codes = self.decode_i32_hex(&encoded_child_codes, reference_rows, "reference-child-codes")?;
		return BayesianCategoricalReferenceSet::from_observations(
			parents,
			child,
			reference_source_rows,
			parent_codes,
			child_codes,
		)
		.map_err(|error| {
			return CheckpointError::manifest(error.to_string());
		});
	}

	/// Resolve exactly one named child field beneath `node`.
	///
	/// # Errors
	///
	/// Returns an error when the node is absent or the field count is not one.
	fn unique_field(&self, node: NodeId, name: &str) -> CheckpointResult<NodeId> {
		let matches = self
			.node(node)?
			.children()
			.iter()
			.copied()
			.filter(|child| {
				return self.node(*child).is_ok_and(|candidate| {
					return candidate.text() == name;
				});
			})
			.collect::<Vec<_>>();
		let &[field] = matches.as_slice() else {
			return Err(CheckpointError::manifest(format!(
				"Bayesian model requires exactly one {name:?} field"
			)));
		};
		return Ok(field);
	}

	/// Decode one categorical node schema.
	///
	/// # Errors
	///
	/// Returns an error when required fields are malformed, label limits are
	/// exceeded, or a hexadecimal payload cannot be decoded.
	fn decode_schema(&mut self, node: NodeId) -> CheckpointResult<BayesianCategoricalSchema> {
		let fields = self.fields(node, &["source-index", "name-bytes", "labels"])?;
		let source_index = Self::parse_usize(self.scalar(fields["source-index"])?, "source-index")?;
		let encoded_name = self.scalar(fields["name-bytes"])?.to_owned();
		let name = self.decode_bytes(&encoded_name, "name-bytes")?;
		let label_nodes = self.node(fields["labels"])?.children().to_vec();
		self.labels = self
			.labels
			.checked_add(label_nodes.len())
			.ok_or_else(|| {
				return CheckpointError::manifest("Bayesian total label count overflowed usize");
			})?;
		if label_nodes.is_empty() || self.labels > self.limits.labels {
			return Err(CheckpointError::manifest(format!(
				"Bayesian total label count is {}, limit is {}",
				self.labels, self.limits.labels
			)));
		}
		let mut dictionary = Vec::with_capacity(label_nodes.len());
		for label in label_nodes {
			if self.node(label)?.text() != "value-bytes" {
				return Err(CheckpointError::manifest(
					"Bayesian labels may contain only value-bytes entries",
				));
			}
			let value = self.scalar(label)?.to_owned();
			dictionary.push(self.decode_bytes(&value, "value-bytes")?);
		}
		return Ok(BayesianCategoricalSchema::from_parts(source_index, name, dictionary));
	}

	/// Index the exact required child fields beneath `node`.
	///
	/// # Errors
	///
	/// Returns an error when a field is unknown, duplicated, missing, or refers to
	/// an absent graph node.
	fn fields(&self, node: NodeId, required: &[&str]) -> CheckpointResult<BTreeMap<String, NodeId>> {
		let allowed = required.iter().copied().collect::<BTreeSet<_>>();
		let mut fields = BTreeMap::new();
		for child in self.node(node)?.children() {
			let name = self.node(*child)?.text();
			if !allowed.contains(name) {
				return Err(CheckpointError::manifest(format!(
					"unknown Bayesian field {name:?}"
				)));
			}
			if fields.insert(name.to_owned(), *child).is_some() {
				return Err(CheckpointError::manifest(format!(
					"duplicate Bayesian field {name:?}"
				)));
			}
		}
		for name in required {
			if !fields.contains_key(*name) {
				return Err(CheckpointError::manifest(format!(
					"missing Bayesian field {name:?}"
				)));
			}
		}
		return Ok(fields);
	}

	/// Borrow a graph node by identity.
	///
	/// # Errors
	///
	/// Returns an error when the graph has no node with the supplied identity.
	fn node(&self, id: NodeId) -> CheckpointResult<&recipe_ogdl::Node> {
		return self.graph
			.node(id)
			.ok_or_else(|| {
				return CheckpointError::manifest("Bayesian OGDL contains an unknown node identity");
			});
	}

	/// Borrow the only scalar value beneath a field node.
	///
	/// # Errors
	///
	/// Returns an error when the field has zero or multiple values, the value has
	/// descendants, or either node identity is absent.
	fn scalar(&self, node: NodeId) -> CheckpointResult<&str> {
		let children = self.node(node)?.children();
		let &[value_node] = children else {
			return Err(CheckpointError::manifest(format!(
				"Bayesian scalar field has {} values",
				children.len()
			)));
		};
		let scalar_node = self.node(value_node)?;
		if !scalar_node.children().is_empty() {
			return Err(CheckpointError::manifest(
				"Bayesian scalar value has descendants",
			));
		}
		return Ok(scalar_node.text());
	}

	/// Parse a canonical decimal `usize` field.
	///
	/// # Errors
	///
	/// Returns an error when the value is not a canonical decimal `usize`.
	fn parse_usize(value: &str, role: &str) -> CheckpointResult<usize> {
		let parsed = value
			.parse::<usize>()
			.map_err(|error| {
				return CheckpointError::manifest(format!("Bayesian {role} is invalid: {error}"));
			})?;
		if parsed.to_string() != value {
			return Err(CheckpointError::manifest(format!(
				"Bayesian {role} is not canonical decimal"
			)));
		}
		return Ok(parsed);
	}

	/// Decode an even-length hexadecimal byte payload.
	///
	/// # Errors
	///
	/// Returns an error when the prefix, length, character encoding, hex digits,
	/// or aggregate payload size is invalid.
	fn decode_bytes(&mut self, value: &str, role: &str) -> CheckpointResult<Vec<u8>> {
		let digits = value
			.strip_prefix("0x")
			.ok_or_else(|| {
				return CheckpointError::manifest(format!("Bayesian {role} lacks 0x prefix"));
			})?;
		if digits.len() % 2 != 0 {
			return Err(CheckpointError::manifest(format!(
				"Bayesian {role} has odd hex length"
			)));
		}
		let bytes = digits.len() / 2;
		self.add_payload(bytes, role)?;
		return digits
			.as_bytes()
			.chunks_exact(2)
			.map(|encoded_chunk| {
				let hex_byte = from_utf8(encoded_chunk).map_err(|error| {
					return CheckpointError::manifest(format!("Bayesian {role} has invalid UTF-8: {error}"));
				})?;
				return u8::from_str_radix(hex_byte, 16).map_err(|error| {
					return CheckpointError::manifest(format!("Bayesian {role} has invalid hex: {error}"));
				});
			})
			.collect();
	}

	/// Decode fixed-width hexadecimal two's-complement categorical codes.
	///
	/// # Errors
	///
	/// Returns an error when the prefix, encoded length, character encoding, hex
	/// digits, or aggregate payload size is invalid.
	fn decode_i32_hex(&mut self, value: &str, count: usize, role: &str) -> CheckpointResult<Vec<i32>> {
		let digits = value
			.strip_prefix("0x")
			.ok_or_else(|| {
				return CheckpointError::manifest(format!("Bayesian {role} lacks 0x prefix"));
			})?;
		let expected = count
			.checked_mul(8)
			.ok_or_else(|| {
				return CheckpointError::manifest(format!("Bayesian {role} length overflowed usize"));
			})?;
		if digits.len() != expected {
			return Err(CheckpointError::manifest(format!(
				"Bayesian {role} has {} hex digits, expected {expected}",
				digits.len()
			)));
		}
		self.add_payload(count.saturating_mul(4), role)?;
		return digits
			.as_bytes()
			.chunks_exact(8)
			.map(|encoded_chunk| {
				let hex_code = from_utf8(encoded_chunk).map_err(|error| {
					return CheckpointError::manifest(format!("Bayesian {role} has invalid UTF-8: {error}"));
				})?;
				let code_bits = u32::from_str_radix(hex_code, 16).map_err(|error| {
					return CheckpointError::manifest(format!("Bayesian {role} has invalid hex: {error}"));
				})?;
				return Ok(code_bits.cast_signed());
			})
			.collect();
	}

	/// Decode fixed-width hexadecimal source-row indices.
	///
	/// # Errors
	///
	/// Returns an error when the prefix, encoded length, character encoding, hex
	/// digits, target width, or aggregate payload size is invalid.
	fn decode_usize_hex(&mut self, value: &str, count: usize, role: &str) -> CheckpointResult<Vec<usize>> {
		let digits = value
			.strip_prefix("0x")
			.ok_or_else(|| {
				return CheckpointError::manifest(format!("Bayesian {role} lacks 0x prefix"));
			})?;
		let expected = count
			.checked_mul(16)
			.ok_or_else(|| {
				return CheckpointError::manifest(format!("Bayesian {role} length overflowed usize"));
			})?;
		if digits.len() != expected {
			return Err(CheckpointError::manifest(format!(
				"Bayesian {role} has {} hex digits, expected {expected}",
				digits.len()
			)));
		}
		self.add_payload(count.saturating_mul(8), role)?;
		return digits
			.as_bytes()
			.chunks_exact(16)
			.map(|encoded_chunk| {
				let hex_index = from_utf8(encoded_chunk).map_err(|error| {
					return CheckpointError::manifest(format!("Bayesian {role} has invalid UTF-8: {error}"));
				})?;
				let source_row = u64::from_str_radix(hex_index, 16).map_err(|error| {
					return CheckpointError::manifest(format!("Bayesian {role} has invalid hex: {error}"));
				})?;
				return usize::try_from(source_row).map_err(|error| {
					return CheckpointError::manifest(format!("Bayesian {role} exceeds usize: {error}"));
				});
			})
			.collect();
	}

	/// Account decoded payload bytes against the aggregate limit.
	///
	/// # Errors
	///
	/// Returns an error when accounting overflows or exceeds the configured limit.
	fn add_payload(&mut self, bytes: usize, role: &str) -> CheckpointResult<()> {
		self.payload_bytes = self.payload_bytes.checked_add(bytes).ok_or_else(|| {
			return CheckpointError::manifest(format!(
				"Bayesian {role} payload accounting overflowed usize"
			));
		})?;
		if self.payload_bytes > self.limits.total_payload_bytes {
			return Err(CheckpointError::manifest(format!(
				"Bayesian payload uses {} bytes, limit is {}",
				self.payload_bytes, self.limits.total_payload_bytes
			)));
		}
		return Ok(());
	}
}
