use std::{
	collections::{BTreeMap, BTreeSet},
	io::Write,
	path::Path,
};

use recipe_ogdl::{Graph, NodeId};

use crate::{
	CheckpointError, CheckpointResult,
	bayes::{
		BayesianCategoricalReferenceSet, BayesianCategoricalSchema, CATEGORICAL_BAYES_SMOOTHING,
		validate_categorical_reference_set,
	},
	checkpoint::atomic_save,
};

const BAYES_MODEL_FORMAT_VERSION_ONE: u32 = 1;
const BAYES_MODEL_FORMAT_VERSION_TWO: u32 = 2;
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
	fn default() -> Self {
		Self {
			source_bytes: 1 << 30,
			nodes: 4_000_000,
			conditionals: 65_536,
			parents: 65_536,
			labels: 1_000_000,
			reference_rows: 100_000_000,
			total_payload_bytes: 1 << 30,
		}
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
	format_version: u32,
	smoothing_bits: u32,
	conditionals: Vec<BayesianCategoricalReferenceSet>,
}

impl BayesModelArtifact {
	pub fn new(references: BayesianCategoricalReferenceSet) -> CheckpointResult<Self> {
		let artifact = Self {
			format_version: BAYES_MODEL_FORMAT_VERSION_ONE,
			smoothing_bits: CATEGORICAL_BAYES_SMOOTHING.to_bits(),
			conditionals: vec![references],
		};
		validate_artifact(&artifact)?;
		Ok(artifact)
	}

	/// Construct the repeated-call observed categorical instrument. A singular
	/// model retains the version-one image; two or more conditionals use the
	/// version-two repeated structure.
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
		Ok(artifact)
	}

	#[must_use]
	pub const fn format_version(&self) -> u32 { self.format_version }

	#[must_use]
	pub const fn smoothing(&self) -> f32 { f32::from_bits(self.smoothing_bits) }

	#[must_use]
	pub fn references(&self) -> &BayesianCategoricalReferenceSet {
		self.conditionals
			.first()
			.expect("a validated Bayesian artifact has at least one conditional")
	}

	#[must_use]
	pub fn conditionals(&self) -> &[BayesianCategoricalReferenceSet] { &self.conditionals }

	/// Continue with another complete observed partition. Repeated rows remain
	/// repeated evidence; saved observations remain before current observations.
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
		for (saved, current) in self.conditionals.iter_mut().zip(current.conditionals) {
			saved.append(current)
				.map_err(|error| CheckpointError::manifest(error.to_string()))?;
		}
		validate_artifact(&self)?;
		Ok(self)
	}

	pub fn encode(&self) -> CheckpointResult<Vec<u8>> {
		validate_artifact(self)?;
		Ok(encode_graph(self)?.to_canonical_string().into_bytes())
	}

	pub fn decode(source: &[u8], limits: BayesModelDecodeLimits) -> CheckpointResult<Self> {
		decode_bayes_model(source, limits)
	}

	pub fn save(&self, path: impl AsRef<Path>) -> CheckpointResult<()> {
		let path = path.as_ref();
		if path.extension().and_then(|extension| extension.to_str()) != Some("ogdl") {
			return Err(CheckpointError::invalid_target(
				path,
				"Bayesian semantic model path must end in .ogdl",
			));
		}
		let encoded = self.encode()?;
		let bytes = u64::try_from(encoded.len()).map_err(|error| {
			CheckpointError::invalid_target(path, format!("Bayesian model size exceeds u64: {error}"))
		})?;
		atomic_save(path, bytes, |file| file.write_all(&encoded))
	}
}

pub fn decode_bayes_model(source: &[u8], limits: BayesModelDecodeLimits) -> CheckpointResult<BayesModelArtifact> {
	Decoder::new(source, limits)?.decode(source)
}

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
			.map_err(|error| CheckpointError::manifest(error.to_string()))?;
	}
	let first = &artifact.conditionals[0];
	let mut children_by_name = BTreeSet::new();
	let mut children_by_source = BTreeSet::new();
	let mut schemas_by_name = BTreeMap::new();
	let mut schemas_by_source = BTreeMap::new();
	for references in &artifact.conditionals {
		if references.reference_rows != first.reference_rows
			|| references.reference_source_rows != first.reference_source_rows
		{
			return Err(CheckpointError::manifest(
				"Bayesian conditionals do not retain the same ordered reference partition",
			));
		}
		if !children_by_name.insert(references.child.name.as_slice())
			|| !children_by_source.insert(references.child.source_index)
		{
			return Err(CheckpointError::manifest(
				"Bayesian conditional children must have unique names and source identities",
			));
		}
		for schema in references
			.parents
			.iter()
			.chain(core::iter::once(&references.child))
		{
			if let Some(existing) = schemas_by_name.insert(schema.name.as_slice(), schema)
				&& existing != schema
			{
				return Err(CheckpointError::manifest(
					"Bayesian repeated node names have inconsistent saved schemas",
				));
			}
			if let Some(existing) = schemas_by_source.insert(schema.source_index, schema)
				&& existing != schema
			{
				return Err(CheckpointError::manifest(
					"Bayesian repeated source identities have inconsistent saved schemas",
				));
			}
		}
	}
	if artifact.conditionals.iter().any(|references| {
		references.parents.iter().any(|parent| {
			children_by_name.contains(parent.name.as_slice()) || children_by_source.contains(&parent.source_index)
		})
	}) {
		return Err(CheckpointError::manifest(
			"the observed multi-output Bayesian instrument does not accept a target child as another conditional's parent",
		));
	}
	Ok(())
}

fn encode_graph(artifact: &BayesModelArtifact) -> CheckpointResult<Graph> {
	let mut graph = Graph::new();
	let root = child_root(&mut graph, ROOT)?;
	field(&mut graph, root, "format-version", artifact.format_version)?;
	field(&mut graph, root, "smoothing", "laplace-one")?;
	if artifact.format_version == BAYES_MODEL_FORMAT_VERSION_ONE {
		encode_reference(&mut graph, root, &artifact.conditionals[0])?;
	} else {
		let conditionals = child(&mut graph, root, "conditionals")?;
		for references in &artifact.conditionals {
			let conditional = child(&mut graph, conditionals, "conditional")?;
			encode_reference(&mut graph, conditional, references)?;
		}
	}
	Ok(graph)
}

fn encode_reference(
	graph: &mut Graph,
	root: NodeId,
	references: &BayesianCategoricalReferenceSet,
) -> CheckpointResult<()> {
	field(graph, root, "reference-rows", references.reference_rows)?;
	field(
		graph,
		root,
		"reference-source-rows",
		encode_usize_hex(&references.reference_source_rows)?,
	)?;
	let parents = child(graph, root, "parents")?;
	for parent in &references.parents {
		encode_schema(graph, parents, "parent", parent)?;
	}
	encode_schema(graph, root, "child", &references.child)?;
	field(
		graph,
		root,
		"reference-parent-codes",
		encode_i32_hex(&references.parent_codes),
	)?;
	field(
		graph,
		root,
		"reference-child-codes",
		encode_i32_hex(&references.child_codes),
	)?;
	Ok(())
}

fn encode_schema(
	graph: &mut Graph,
	parent: NodeId,
	tag: &str,
	schema: &BayesianCategoricalSchema,
) -> CheckpointResult<()> {
	let node = child(graph, parent, tag)?;
	field(graph, node, "source-index", schema.source_index)?;
	field(graph, node, "name-bytes", encode_bytes(&schema.name))?;
	let labels = child(graph, node, "labels")?;
	for label in &schema.dictionary {
		field(graph, labels, "value-bytes", encode_bytes(label))?;
	}
	Ok(())
}

fn child_root(graph: &mut Graph, text: &str) -> CheckpointResult<NodeId> {
	graph.append_root(text)
		.map_err(|error| CheckpointError::manifest(format!("encode Bayesian root: {error}")))
}

fn child(graph: &mut Graph, parent: NodeId, text: impl Into<String>) -> CheckpointResult<NodeId> {
	graph.append_child(parent, text)
		.map_err(|error| CheckpointError::manifest(format!("encode Bayesian OGDL node: {error}")))
}

fn field(graph: &mut Graph, parent: NodeId, name: impl Into<String>, value: impl ToString) -> CheckpointResult<()> {
	let field = child(graph, parent, name)?;
	child(graph, field, value.to_string())?;
	Ok(())
}

fn encode_bytes(bytes: &[u8]) -> String {
	let mut output = String::with_capacity(2 + bytes.len().saturating_mul(2));
	output.push_str("0x");
	for byte in bytes {
		use core::fmt::Write as _;
		write!(output, "{byte:02x}").expect("writing to String cannot fail");
	}
	output
}

fn encode_i32_hex(values: &[i32]) -> String {
	let mut output = String::with_capacity(2 + values.len().saturating_mul(8));
	output.push_str("0x");
	for value in values {
		use core::fmt::Write as _;
		write!(output, "{:08x}", *value as u32).expect("writing to String cannot fail");
	}
	output
}

fn encode_usize_hex(values: &[usize]) -> CheckpointResult<String> {
	let mut output = String::with_capacity(2 + values.len().saturating_mul(16));
	output.push_str("0x");
	for value in values {
		let value = u64::try_from(*value)
			.map_err(|error| CheckpointError::manifest(format!("Bayesian source row exceeds u64: {error}")))?;
		use core::fmt::Write as _;
		write!(output, "{value:016x}").expect("writing to String cannot fail");
	}
	Ok(output)
}

#[derive(Debug)]
struct Decoder {
	graph: Graph,
	limits: BayesModelDecodeLimits,
	payload_bytes: usize,
	labels: usize,
	parents: usize,
	reference_rows: usize,
}

impl Decoder {
	fn new(source: &[u8], limits: BayesModelDecodeLimits) -> CheckpointResult<Self> {
		if source.len() > limits.source_bytes {
			return Err(CheckpointError::manifest(format!(
				"Bayesian model source has {} bytes, limit is {}",
				source.len(),
				limits.source_bytes
			)));
		}
		let text = core::str::from_utf8(source)
			.map_err(|error| CheckpointError::manifest(format!("Bayesian model is not UTF-8: {error}")))?;
		let graph = Graph::parse(text)
			.map_err(|error| CheckpointError::manifest(format!("invalid Bayesian OGDL: {error}")))?;
		if graph.len() > limits.nodes {
			return Err(CheckpointError::manifest(format!(
				"Bayesian model has {} nodes, limit is {}",
				graph.len(),
				limits.nodes
			)));
		}
		Ok(Self {
			graph,
			limits,
			payload_bytes: 0,
			labels: 0,
			parents: 0,
			reference_rows: 0,
		})
	}

	fn decode(mut self, source: &[u8]) -> CheckpointResult<BayesModelArtifact> {
		let [root] = self.graph.roots() else {
			return Err(CheckpointError::manifest(format!(
				"Bayesian model requires exactly one root, found {}",
				self.graph.roots().len()
			)));
		};
		if self.node(*root)?.text() != ROOT {
			return Err(CheckpointError::manifest(format!(
				"Bayesian model root must be {ROOT:?}"
			)));
		}
		let format_field = self.unique_field(*root, "format-version")?;
		let format_version = self.parse_u32(self.scalar(format_field)?, "format-version")?;
		let conditionals = match format_version {
			BAYES_MODEL_FORMAT_VERSION_ONE => {
				let fields = self.fields(*root, &[
					"format-version",
					"smoothing",
					"reference-rows",
					"reference-source-rows",
					"parents",
					"child",
					"reference-parent-codes",
					"reference-child-codes",
				])?;
				self.require_laplace_one(fields["smoothing"])?;
				vec![self.decode_reference_fields(&fields)?]
			}
			BAYES_MODEL_FORMAT_VERSION_TWO => {
				let fields = self.fields(*root, &["format-version", "smoothing", "conditionals"])?;
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
		Ok(artifact)
	}

	fn require_laplace_one(&self, field: NodeId) -> CheckpointResult<()> {
		if self.scalar(field)? != "laplace-one" {
			return Err(CheckpointError::manifest(
				"Bayesian smoothing must be the canonical laplace-one contract",
			));
		}
		Ok(())
	}

	fn decode_reference(&mut self, node: NodeId) -> CheckpointResult<BayesianCategoricalReferenceSet> {
		let fields = self.fields(node, &[
			"reference-rows",
			"reference-source-rows",
			"parents",
			"child",
			"reference-parent-codes",
			"reference-child-codes",
		])?;
		self.decode_reference_fields(&fields)
	}

	fn decode_reference_fields(
		&mut self,
		fields: &BTreeMap<String, NodeId>,
	) -> CheckpointResult<BayesianCategoricalReferenceSet> {
		let reference_rows = self.parse_usize(self.scalar(fields["reference-rows"])?, "reference-rows")?;
		self.reference_rows = self
			.reference_rows
			.checked_add(reference_rows)
			.ok_or_else(|| {
				CheckpointError::manifest("Bayesian aggregate reference row count overflowed usize")
			})?;
		if reference_rows == 0 || self.reference_rows > self.limits.reference_rows {
			return Err(CheckpointError::manifest(format!(
				"Bayesian aggregate reference row count is {}, limit is {}",
				self.reference_rows, self.limits.reference_rows
			)));
		}
		let parent_nodes = self.node(fields["parents"])?.children().to_vec();
		self.parents = self
			.parents
			.checked_add(parent_nodes.len())
			.ok_or_else(|| CheckpointError::manifest("Bayesian aggregate parent count overflowed usize"))?;
		if parent_nodes.is_empty() || self.parents > self.limits.parents {
			return Err(CheckpointError::manifest(format!(
				"Bayesian aggregate parent count is {}, limit is {}",
				self.parents, self.limits.parents
			)));
		}
		let mut parents = Vec::with_capacity(parent_nodes.len());
		for parent in parent_nodes {
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
			CheckpointError::manifest("Bayesian reference parent element count overflowed usize")
		})?;
		let parent_codes = self.scalar(fields["reference-parent-codes"])?.to_owned();
		let parent_codes = self.decode_i32_hex(&parent_codes, parent_elements, "reference-parent-codes")?;
		let child_codes = self.scalar(fields["reference-child-codes"])?.to_owned();
		let child_codes = self.decode_i32_hex(&child_codes, reference_rows, "reference-child-codes")?;
		let mut parent_cardinalities = Vec::with_capacity(parents.len());
		for parent in &parents {
			let cardinality =
				parent.dictionary.len().checked_add(1).ok_or_else(|| {
					CheckpointError::manifest("Bayesian parent cardinality overflowed usize")
				})?;
			parent_cardinalities.push(i32::try_from(cardinality).map_err(|error| {
				CheckpointError::manifest(format!(
					"Bayesian parent cardinality exceeds int32: {error}"
				))
			})?);
		}
		let (parent_multipliers, parent_configurations) = mixed_radix(&parent_cardinalities)?;
		Ok(BayesianCategoricalReferenceSet {
			parents,
			child,
			reference_source_rows,
			reference_rows,
			parent_codes,
			child_codes,
			parent_cardinalities,
			parent_multipliers,
			parent_configurations,
		})
	}

	fn unique_field(&self, node: NodeId, name: &str) -> CheckpointResult<NodeId> {
		let matches = self
			.node(node)?
			.children()
			.iter()
			.copied()
			.filter(|child| self.node(*child).is_ok_and(|node| node.text() == name))
			.collect::<Vec<_>>();
		let [field] = matches.as_slice() else {
			return Err(CheckpointError::manifest(format!(
				"Bayesian model requires exactly one {name:?} field"
			)));
		};
		Ok(*field)
	}

	fn decode_schema(&mut self, node: NodeId) -> CheckpointResult<BayesianCategoricalSchema> {
		let fields = self.fields(node, &["source-index", "name-bytes", "labels"])?;
		let source_index = self.parse_usize(self.scalar(fields["source-index"])?, "source-index")?;
		let name = self.scalar(fields["name-bytes"])?.to_owned();
		let name = self.decode_bytes(&name, "name-bytes")?;
		let label_nodes = self.node(fields["labels"])?.children().to_vec();
		self.labels = self
			.labels
			.checked_add(label_nodes.len())
			.ok_or_else(|| CheckpointError::manifest("Bayesian total label count overflowed usize"))?;
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
		Ok(BayesianCategoricalSchema {
			source_index,
			name,
			dictionary,
		})
	}

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
		Ok(fields)
	}

	fn node(&self, id: NodeId) -> CheckpointResult<&recipe_ogdl::Node> {
		self.graph
			.node(id)
			.ok_or_else(|| CheckpointError::manifest("Bayesian OGDL contains an unknown node identity"))
	}

	fn scalar(&self, node: NodeId) -> CheckpointResult<&str> {
		let children = self.node(node)?.children();
		let [value] = children else {
			return Err(CheckpointError::manifest(format!(
				"Bayesian scalar field has {} values",
				children.len()
			)));
		};
		let value = self.node(*value)?;
		if !value.children().is_empty() {
			return Err(CheckpointError::manifest(
				"Bayesian scalar value has descendants",
			));
		}
		Ok(value.text())
	}

	fn parse_u32(&self, value: &str, role: &str) -> CheckpointResult<u32> {
		let parsed = value
			.parse::<u32>()
			.map_err(|error| CheckpointError::manifest(format!("Bayesian {role} is invalid: {error}")))?;
		if parsed.to_string() != value {
			return Err(CheckpointError::manifest(format!(
				"Bayesian {role} is not canonical decimal"
			)));
		}
		Ok(parsed)
	}

	fn parse_usize(&self, value: &str, role: &str) -> CheckpointResult<usize> {
		let parsed = value
			.parse::<usize>()
			.map_err(|error| CheckpointError::manifest(format!("Bayesian {role} is invalid: {error}")))?;
		if parsed.to_string() != value {
			return Err(CheckpointError::manifest(format!(
				"Bayesian {role} is not canonical decimal"
			)));
		}
		Ok(parsed)
	}

	fn decode_bytes(&mut self, value: &str, role: &str) -> CheckpointResult<Vec<u8>> {
		let digits = value
			.strip_prefix("0x")
			.ok_or_else(|| CheckpointError::manifest(format!("Bayesian {role} lacks 0x prefix")))?;
		if digits.len() % 2 != 0 {
			return Err(CheckpointError::manifest(format!(
				"Bayesian {role} has odd hex length"
			)));
		}
		let bytes = digits.len() / 2;
		self.add_payload(bytes, role)?;
		(0..bytes)
			.map(|index| decode_hex_byte(&digits[index * 2..index * 2 + 2], role))
			.collect()
	}

	fn decode_i32_hex(&mut self, value: &str, count: usize, role: &str) -> CheckpointResult<Vec<i32>> {
		let digits = value
			.strip_prefix("0x")
			.ok_or_else(|| CheckpointError::manifest(format!("Bayesian {role} lacks 0x prefix")))?;
		let expected = count
			.checked_mul(8)
			.ok_or_else(|| CheckpointError::manifest(format!("Bayesian {role} length overflowed usize")))?;
		if digits.len() != expected {
			return Err(CheckpointError::manifest(format!(
				"Bayesian {role} has {} hex digits, expected {expected}",
				digits.len()
			)));
		}
		self.add_payload(count.checked_mul(4).unwrap_or(usize::MAX), role)?;
		(0..count)
			.map(|index| {
				u32::from_str_radix(&digits[index * 8..index * 8 + 8], 16)
					.map(|value| value as i32)
					.map_err(|error| {
						CheckpointError::manifest(format!("Bayesian {role} has invalid hex: {error}"))
					})
			})
			.collect()
	}

	fn decode_usize_hex(&mut self, value: &str, count: usize, role: &str) -> CheckpointResult<Vec<usize>> {
		let digits = value
			.strip_prefix("0x")
			.ok_or_else(|| CheckpointError::manifest(format!("Bayesian {role} lacks 0x prefix")))?;
		let expected = count
			.checked_mul(16)
			.ok_or_else(|| CheckpointError::manifest(format!("Bayesian {role} length overflowed usize")))?;
		if digits.len() != expected {
			return Err(CheckpointError::manifest(format!(
				"Bayesian {role} has {} hex digits, expected {expected}",
				digits.len()
			)));
		}
		self.add_payload(count.checked_mul(8).unwrap_or(usize::MAX), role)?;
		(0..count)
			.map(|index| {
				let value = u64::from_str_radix(&digits[index * 16..index * 16 + 16], 16).map_err(|error| {
					CheckpointError::manifest(format!("Bayesian {role} has invalid hex: {error}"))
				})?;
				usize::try_from(value).map_err(|error| {
					CheckpointError::manifest(format!("Bayesian {role} exceeds usize: {error}"))
				})
			})
			.collect()
	}

	fn add_payload(&mut self, bytes: usize, role: &str) -> CheckpointResult<()> {
		self.payload_bytes = self.payload_bytes.checked_add(bytes).ok_or_else(|| {
			CheckpointError::manifest(format!(
				"Bayesian {role} payload accounting overflowed usize"
			))
		})?;
		if self.payload_bytes > self.limits.total_payload_bytes {
			return Err(CheckpointError::manifest(format!(
				"Bayesian payload uses {} bytes, limit is {}",
				self.payload_bytes, self.limits.total_payload_bytes
			)));
		}
		Ok(())
	}
}

fn decode_hex_byte(value: &str, role: &str) -> CheckpointResult<u8> {
	u8::from_str_radix(value, 16)
		.map_err(|error| CheckpointError::manifest(format!("Bayesian {role} has invalid hex: {error}")))
}

fn mixed_radix(cardinalities: &[i32]) -> CheckpointResult<(Vec<i32>, u64)> {
	let mut configurations = 1_u64;
	let mut multipliers = vec![0_i32; cardinalities.len()];
	for (index, cardinality) in cardinalities.iter().copied().enumerate().rev() {
		let cardinality = u64::try_from(cardinality)
			.map_err(|_| CheckpointError::manifest("Bayesian parent cardinality is negative"))?;
		if cardinality == 0 {
			return Err(CheckpointError::manifest(
				"Bayesian parent cardinality is zero",
			));
		}
		multipliers[index] = i32::try_from(configurations).map_err(|error| {
			CheckpointError::manifest(format!("Bayesian parent multiplier exceeds int32: {error}"))
		})?;
		configurations = configurations
			.checked_mul(cardinality)
			.ok_or_else(|| CheckpointError::manifest("Bayesian parent configurations overflowed u64"))?;
	}
	if configurations > i32::MAX as u64 {
		return Err(CheckpointError::manifest(
			"Bayesian parent configuration count exceeds int32",
		));
	}
	Ok((multipliers, configurations))
}
