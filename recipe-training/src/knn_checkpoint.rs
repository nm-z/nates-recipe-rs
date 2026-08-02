use core::num::NonZeroU64;
use std::{
	collections::{BTreeMap, BTreeSet},
	io::Write,
	path::Path,
};

use recipe_ingest::{
	EncodedImageFormat, ImageColorModel, ImageValueLayout, ImageValueRange, SemanticType, VectorEncoding, VectorRole,
};
use recipe_ogdl::{Graph, NodeId};

use crate::{
	CheckpointArtifactMetadata, CheckpointArtifactVector, CheckpointDecodeErrorKind, CheckpointError,
	CheckpointImageMetadata, CheckpointPath, CheckpointResult, CompiledFeatureSpan, DenseDataNormalization,
	DenseFeatureLowering, DenseOperation, KnnLabelValue, KnnReferenceOutput, KnnReferenceSet, KnnReferenceValues,
	checkpoint::{atomic_save, decode_error, validate_saved_vector},
};

const KNN_MODEL_FORMAT_VERSION: u32 = 1;
const ROOT: &str = "recipe-knn-model";

/// Finite allocation and structural bounds for one semantic KNN model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnnModelDecodeLimits {
	pub source_bytes: usize,
	pub nodes: usize,
	pub vectors: usize,
	pub feature_spans: usize,
	pub reference_rows: usize,
	pub feature_width: usize,
	pub outputs: usize,
	pub labels: usize,
	pub metadata_entries: usize,
	pub total_payload_bytes: usize,
}

impl Default for KnnModelDecodeLimits {
	fn default() -> Self {
		Self {
			source_bytes: 1 << 30,
			nodes: 4_000_000,
			vectors: 65_536,
			feature_spans: 65_536,
			reference_rows: 100_000_000,
			feature_width: 1_000_000,
			outputs: 65_536,
			labels: 1_000_000,
			metadata_entries: 1_000_000,
			total_payload_bytes: 1 << 30,
		}
	}
}

/// Complete semantic artifact for one all-output KNN model.
///
/// The reference set contains no opaque implementation state: it retains the
/// exact prepared feature image, every independently masked target, and every
/// decoder required to recover nonnumeric predictions. Normalization and
/// post-reduction declarations are stored even though their public execution
/// policy is deliberately not guessed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnModelArtifact {
	format_version: u32,
	references: KnnReferenceSet,
	data_normalization: Option<DenseDataNormalization>,
	operations: Vec<DenseOperation>,
}

impl KnnModelArtifact {
	pub fn new(
		references: KnnReferenceSet,
		data_normalization: Option<DenseDataNormalization>,
		operations: impl IntoIterator<Item = DenseOperation>,
	) -> CheckpointResult<Self> {
		let artifact = Self {
			format_version: KNN_MODEL_FORMAT_VERSION,
			references,
			data_normalization,
			operations: operations.into_iter().collect(),
		};
		validate_artifact(&artifact)?;
		Ok(artifact)
	}

	#[must_use]
	pub const fn format_version(&self) -> u32 { self.format_version }

	#[must_use]
	pub const fn references(&self) -> &KnnReferenceSet { &self.references }

	#[must_use]
	pub const fn data_normalization(&self) -> Option<DenseDataNormalization> { self.data_normalization }

	#[must_use]
	pub fn operations(&self) -> &[DenseOperation] { &self.operations }

	/// Continue one KNN model with another prepared training partition.
	///
	/// The saved references remain first and the newly prepared references are
	/// appended in their retained row order. Repeated observations are retained:
	/// KNN has no global row identity with which to deduplicate them, and their
	/// multiplicity is part of the declared data. This order is also the stable
	/// distance-tie order. The current declaration must retain the saved
	/// neighbors, row-free vector schema, feature lowering, normalization, and
	/// post-reduction topology.
	pub fn continue_with(mut self, current: Self) -> CheckpointResult<Self> {
		validate_artifact(&self)?;
		validate_artifact(&current)?;
		validate_resume_compatibility(&self, &current)?;
		append_reference_set(&mut self.references, current.references)?;
		validate_artifact(&self)?;
		Ok(self)
	}

	/// Encode one strict canonical textual OGDL model image.
	pub fn encode(&self) -> CheckpointResult<Vec<u8>> {
		validate_artifact(self)?;
		let graph = encode_graph(self)?;
		Ok(graph.to_canonical_string().into_bytes())
	}

	/// Decode and validate one bounded semantic KNN model image.
	pub fn decode(source: &[u8], limits: KnnModelDecodeLimits) -> CheckpointResult<Self> {
		decode_knn_model(source, limits)
	}

	/// Atomically persist this semantic model as its canonical textual OGDL
	/// image. The KNN reference state contains no native kernel bytes.
	pub fn save(&self, path: impl AsRef<Path>) -> CheckpointResult<()> {
		let path = path.as_ref();
		if path.extension().and_then(|extension| extension.to_str()) != Some("ogdl") {
			return Err(CheckpointError::invalid_target(
				path,
				"KNN semantic model path must end in .ogdl",
			));
		}
		let encoded = self.encode()?;
		let encoded_bytes = u64::try_from(encoded.len()).map_err(|error| {
			CheckpointError::invalid_target(path, format!("KNN model size exceeds u64: {error}"))
		})?;
		atomic_save(path, encoded_bytes, |file| file.write_all(&encoded))
	}
}

pub fn decode_knn_model(source: &[u8], limits: KnnModelDecodeLimits) -> CheckpointResult<KnnModelArtifact> {
	Decoder::new(source, limits)?.decode()
}

fn encode_graph(artifact: &KnnModelArtifact) -> CheckpointResult<Graph> {
	let mut graph = Graph::new();
	let root = graph
		.append_root(ROOT)
		.map_err(|error| CheckpointError::manifest(format!("encode KNN root: {error}")))?;
	field(&mut graph, root, "format-version", artifact.format_version)?;
	field(
		&mut graph,
		root,
		"neighbors",
		artifact.references.neighbors.get(),
	)?;
	field(
		&mut graph,
		root,
		"data-normalization",
		artifact
			.data_normalization
			.map_or("none", data_normalization),
	)?;

	let operations = child(&mut graph, root, "operations")?;
	for operation in &artifact.operations {
		field(
			&mut graph,
			operations,
			"operation",
			dense_operation(*operation),
		)?;
	}

	let vectors = child(&mut graph, root, "vectors")?;
	for vector in &artifact.references.vectors {
		encode_vector(&mut graph, vectors, vector)?;
	}

	let spans = child(&mut graph, root, "feature-spans")?;
	for span in &artifact.references.feature_spans {
		encode_span(&mut graph, spans, span)?;
	}

	let mask = child(&mut graph, root, "normalization-mask")?;
	match &artifact.references.normalization_mask {
		Some(bits) => {
			let tag = child(&mut graph, mask, "f32-bits")?;
			scalar(&mut graph, tag, encode_u32_hex(bits))?;
		}
		None => {
			child(&mut graph, mask, "none")?;
		}
	}

	let shape = child(&mut graph, root, "reference-shape")?;
	field(
		&mut graph,
		shape,
		"rows",
		artifact.references.reference_rows,
	)?;
	field(
		&mut graph,
		shape,
		"feature-width",
		artifact.references.feature_width,
	)?;
	field(
		&mut graph,
		root,
		"reference-source-rows",
		encode_usize_hex(&artifact.references.reference_source_rows)?,
	)?;
	field(
		&mut graph,
		root,
		"reference-feature-f32-bits",
		encode_u32_hex(&artifact.references.reference_feature_bits),
	)?;

	let outputs = child(&mut graph, root, "outputs")?;
	for output in &artifact.references.outputs {
		encode_output(&mut graph, outputs, output)?;
	}
	Ok(graph)
}

fn encode_vector(graph: &mut Graph, parent: NodeId, vector: &CheckpointArtifactVector) -> CheckpointResult<()> {
	let node = child(graph, parent, "vector")?;
	field(graph, node, "source-index", vector.source_index())?;
	field(graph, node, "name-bytes", encode_bytes(vector.name()))?;
	field(graph, node, "role", vector_role(vector.role()))?;
	field(
		graph,
		node,
		"semantic-type",
		semantic_type(vector.semantic_type()),
	)?;
	field(graph, node, "encoding", vector_encoding(vector.encoding()))?;
	encode_metadata(graph, node, vector.metadata())
}

fn encode_metadata(graph: &mut Graph, parent: NodeId, metadata: &CheckpointArtifactMetadata) -> CheckpointResult<()> {
	let node = child(graph, parent, "metadata")?;
	match metadata {
		CheckpointArtifactMetadata::None => {
			child(graph, node, "none")?;
		}
		CheckpointArtifactMetadata::Temporal {
			unix_seconds,
			nanoseconds,
		} => {
			let tag = child(graph, node, "temporal")?;
			field(graph, tag, "unix-seconds", *unix_seconds)?;
			field(graph, tag, "nanoseconds", *nanoseconds)?;
		}
		CheckpointArtifactMetadata::Categorical { dictionary } => {
			let tag = child(graph, node, "categorical")?;
			for value in dictionary {
				field(graph, tag, "value-bytes", encode_bytes(value))?;
			}
		}
		CheckpointArtifactMetadata::Ordinal { ordered_labels } => {
			let tag = child(graph, node, "ordinal")?;
			for value in ordered_labels {
				field(graph, tag, "value-bytes", encode_bytes(value))?;
			}
		}
		CheckpointArtifactMetadata::Image { encoded_variants } => {
			let tag = child(graph, node, "image")?;
			for variant in encoded_variants {
				let item = child(graph, tag, "variant")?;
				field(graph, item, "format", image_format(variant.format()))?;
				field(graph, item, "width", variant.width())?;
				field(graph, item, "height", variant.height())?;
				field(
					graph,
					item,
					"channels",
					variant
						.channels()
						.map_or_else(|| "none".to_owned(), |value| value.to_string()),
				)?;
				field(
					graph,
					item,
					"color-model",
					variant.color_model().map_or("none", image_color_model),
				)?;
				field(
					graph,
					item,
					"sample-bits",
					variant
						.sample_bits()
						.map_or_else(|| "none".to_owned(), |value| value.to_string()),
				)?;
				field(
					graph,
					item,
					"value-layout",
					image_value_layout(variant.value_layout()),
				)?;
				field(
					graph,
					item,
					"value-range",
					image_value_range(variant.value_range()),
				)?;
			}
		}
	}
	Ok(())
}

fn encode_span(graph: &mut Graph, parent: NodeId, span: &CompiledFeatureSpan) -> CheckpointResult<()> {
	let node = child(graph, parent, "span")?;
	field(graph, node, "source-index", span.source_vector())?;
	field(graph, node, "start", span.start())?;
	field(graph, node, "width", span.width())?;
	let lowering = child(graph, node, "lowering")?;
	match span.lowering() {
		DenseFeatureLowering::NumericScalar => {
			child(graph, lowering, "numeric-scalar")?;
		}
		DenseFeatureLowering::CategoricalOneHot {
			dictionary_width,
			reserved_index,
		} => {
			let tag = child(graph, lowering, "categorical-one-hot")?;
			field(graph, tag, "dictionary-width", dictionary_width)?;
			field(graph, tag, "reserved-index", reserved_index)?;
		}
	}
	Ok(())
}

fn encode_output(graph: &mut Graph, parent: NodeId, output: &KnnReferenceOutput) -> CheckpointResult<()> {
	let node = child(graph, parent, "output")?;
	field(graph, node, "source-index", output.schema.source_index())?;
	field(graph, node, "known-mask", encode_known(&output.known))?;
	let values = child(graph, node, "values")?;
	match &output.values {
		KnnReferenceValues::NumericF32Bits(bits) => {
			let tag = child(graph, values, "numeric-f32-bits")?;
			scalar(graph, tag, encode_u32_hex(bits))?;
		}
		KnnReferenceValues::DiscreteI32 { codes, labels } => {
			let tag = child(graph, values, "discrete-int32")?;
			field(graph, tag, "codes", encode_i32_hex(codes))?;
			let dictionary = child(graph, tag, "labels")?;
			for label in labels {
				match label {
					KnnLabelValue::I32(value) => field(graph, dictionary, "int32", *value)?,
					KnnLabelValue::Bytes(value) => field(graph, dictionary, "bytes", encode_bytes(value))?,
				}
			}
		}
	}
	Ok(())
}

fn child(graph: &mut Graph, parent: NodeId, text: impl Into<String>) -> CheckpointResult<NodeId> {
	graph.append_child(parent, text)
		.map_err(|error| CheckpointError::manifest(format!("encode KNN OGDL node: {error}")))
}

fn scalar(graph: &mut Graph, parent: NodeId, value: impl ToString) -> CheckpointResult<NodeId> {
	child(graph, parent, value.to_string())
}

fn field(graph: &mut Graph, parent: NodeId, name: impl Into<String>, value: impl ToString) -> CheckpointResult<()> {
	let node = child(graph, parent, name)?;
	scalar(graph, node, value)?;
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

fn encode_u32_hex(values: &[u32]) -> String {
	let mut output = String::with_capacity(2 + values.len().saturating_mul(8));
	output.push_str("0x");
	for value in values {
		use core::fmt::Write as _;
		write!(output, "{value:08x}").expect("writing to String cannot fail");
	}
	output
}

fn encode_i32_hex(values: &[i32]) -> String {
	encode_u32_hex(&values.iter().map(|value| *value as u32).collect::<Vec<_>>())
}

fn encode_usize_hex(values: &[usize]) -> CheckpointResult<String> {
	let mut output = String::with_capacity(2 + values.len().saturating_mul(16));
	output.push_str("0x");
	for value in values {
		let value = u64::try_from(*value)
			.map_err(|error| CheckpointError::manifest(format!("KNN source row does not fit u64: {error}")))?;
		use core::fmt::Write as _;
		write!(output, "{value:016x}").expect("writing to String cannot fail");
	}
	Ok(output)
}

fn encode_known(values: &[i32]) -> String {
	let mut output = String::with_capacity(2 + values.len());
	output.push_str("0b");
	for value in values {
		output.push(if *value == 1 { '1' } else { '0' });
	}
	output
}

fn validate_artifact(artifact: &KnnModelArtifact) -> CheckpointResult<()> {
	if artifact.format_version != KNN_MODEL_FORMAT_VERSION {
		return Err(CheckpointError::manifest(format!(
			"KNN model format version {} is not supported",
			artifact.format_version
		)));
	}
	let references = &artifact.references;
	if references.reference_rows == 0 || references.feature_width == 0 {
		return Err(CheckpointError::manifest(
			"KNN reference rows and feature width must both be nonzero",
		));
	}
	let feature_elements = references
		.reference_rows
		.checked_mul(references.feature_width)
		.ok_or_else(|| CheckpointError::manifest("KNN reference feature shape overflowed usize"))?;
	if references.reference_feature_bits.len() != feature_elements {
		return Err(CheckpointError::manifest(format!(
			"KNN reference feature image has {} elements, expected {feature_elements}",
			references.reference_feature_bits.len()
		)));
	}
	if references
		.reference_feature_bits
		.iter()
		.any(|bits| !f32::from_bits(*bits).is_finite())
	{
		return Err(CheckpointError::manifest(
			"KNN reference features contain a non-finite f32",
		));
	}
	if references.reference_source_rows.len() != references.reference_rows {
		return Err(CheckpointError::manifest(
			"KNN reference-source-row count differs from the reference row count",
		));
	}
	validate_vectors(references)?;
	validate_spans(references)?;
	validate_mask(references)?;
	validate_outputs(references)
}

fn validate_resume_compatibility(saved: &KnnModelArtifact, current: &KnnModelArtifact) -> CheckpointResult<()> {
	let saved_references = &saved.references;
	let current_references = &current.references;
	if saved.format_version != current.format_version
		|| saved_references.neighbors != current_references.neighbors
		|| saved.data_normalization != current.data_normalization
		|| saved.operations != current.operations
	{
		return Err(incompatible_resume(
			"saved and current KNN neighbor count, normalization, or topology differ",
		));
	}
	if saved_references.vectors != current_references.vectors
		|| saved_references.feature_spans != current_references.feature_spans
		|| saved_references.normalization_mask != current_references.normalization_mask
		|| saved_references.feature_width != current_references.feature_width
	{
		return Err(incompatible_resume(
			"saved and current KNN row-free vector schemas or feature lowering differ",
		));
	}
	if saved_references.outputs.len() != current_references.outputs.len()
		|| saved_references
			.outputs
			.iter()
			.zip(&current_references.outputs)
			.any(|(saved, current)| saved.schema != current.schema)
	{
		return Err(incompatible_resume(
			"saved and current KNN declared output schemas or order differ",
		));
	}
	Ok(())
}

fn append_reference_set(saved: &mut KnnReferenceSet, current: KnnReferenceSet) -> CheckpointResult<()> {
	let combined_rows = saved
		.reference_rows
		.checked_add(current.reference_rows)
		.ok_or_else(|| incompatible_resume("combined KNN reference row count overflowed usize"))?;
	let combined_feature_elements = combined_rows
		.checked_mul(saved.feature_width)
		.ok_or_else(|| incompatible_resume("combined KNN reference feature shape overflowed usize"))?;
	saved.reference_source_rows
		.try_reserve_exact(current.reference_source_rows.len())
		.map_err(|error| incompatible_resume(format!("reserve combined KNN source rows: {error}")))?;
	saved.reference_feature_bits
		.try_reserve_exact(current.reference_feature_bits.len())
		.map_err(|error| incompatible_resume(format!("reserve combined KNN feature image: {error}")))?;

	saved.reference_source_rows
		.extend(current.reference_source_rows);
	saved.reference_feature_bits
		.extend(current.reference_feature_bits);
	for (saved_output, current_output) in saved.outputs.iter_mut().zip(current.outputs) {
		append_output(saved_output, current_output)?;
	}
	saved.reference_rows = combined_rows;
	if saved.reference_feature_bits.len() != combined_feature_elements {
		return Err(incompatible_resume(
			"combined KNN reference feature image differs from its checked shape",
		));
	}
	Ok(())
}

fn append_output(saved: &mut KnnReferenceOutput, current: KnnReferenceOutput) -> CheckpointResult<()> {
	let combined_known = saved
		.known_references
		.checked_add(current.known_references)
		.ok_or_else(|| incompatible_resume("combined KNN known-reference count overflowed u64"))?;
	saved.known
		.try_reserve_exact(current.known.len())
		.map_err(|error| incompatible_resume(format!("reserve combined KNN known mask: {error}")))?;
	saved.known.extend(current.known);
	saved.known_references = combined_known;

	match (&mut saved.values, current.values) {
		(KnnReferenceValues::NumericF32Bits(saved_bits), KnnReferenceValues::NumericF32Bits(current_bits)) => {
			saved_bits
				.try_reserve_exact(current_bits.len())
				.map_err(|error| {
					incompatible_resume(format!("reserve combined KNN numeric output: {error}"))
				})?;
			saved_bits.extend(current_bits);
		}
		(
			KnnReferenceValues::DiscreteI32 {
				codes: saved_codes,
				labels: saved_labels,
			},
			KnnReferenceValues::DiscreteI32 {
				codes: current_codes,
				labels: current_labels,
			},
		) => append_discrete_output(saved_codes, saved_labels, current_codes, current_labels)?,
		_ => {
			return Err(incompatible_resume(
				"saved and current KNN output aggregation kinds differ",
			));
		}
	}
	Ok(())
}

fn append_discrete_output(
	saved_codes: &mut Vec<i32>,
	saved_labels: &mut Vec<KnnLabelValue>,
	current_codes: Vec<i32>,
	current_labels: Vec<KnnLabelValue>,
) -> CheckpointResult<()> {
	let mut codes_by_label = saved_labels
		.iter()
		.cloned()
		.enumerate()
		.map(|(index, label)| {
			let code = i32::try_from(index).map_err(|error| {
				incompatible_resume(format!("saved KNN label index does not fit int32: {error}"))
			})?;
			Ok((label, code))
		})
		.collect::<CheckpointResult<BTreeMap<_, _>>>()?;
	for label in &current_labels {
		if codes_by_label.contains_key(label) {
			continue;
		}
		let code = i32::try_from(saved_labels.len()).map_err(|error| {
			incompatible_resume(format!(
				"combined KNN label index does not fit int32: {error}"
			))
		})?;
		saved_labels
			.try_reserve_exact(1)
			.map_err(|error| incompatible_resume(format!("reserve combined KNN label dictionary: {error}")))?;
		saved_labels.push(label.clone());
		codes_by_label.insert(label.clone(), code);
	}
	saved_codes
		.try_reserve_exact(current_codes.len())
		.map_err(|error| incompatible_resume(format!("reserve combined KNN discrete output: {error}")))?;
	for code in current_codes {
		let index = usize::try_from(code)
			.map_err(|error| incompatible_resume(format!("current KNN class code is negative: {error}")))?;
		let label = current_labels.get(index).ok_or_else(|| {
			incompatible_resume(format!(
				"current KNN class code {code} is outside its dictionary"
			))
		})?;
		let mapped = codes_by_label
			.get(label)
			.copied()
			.ok_or_else(|| incompatible_resume("combined KNN label has no class code"))?;
		saved_codes.push(mapped);
	}
	Ok(())
}

fn incompatible_resume(detail: impl Into<String>) -> CheckpointError {
	CheckpointError::IncompatibleResume {
		detail: detail.into(),
	}
}

fn validate_vectors(references: &KnnReferenceSet) -> CheckpointResult<()> {
	if references.vectors.is_empty() {
		return Err(CheckpointError::manifest(
			"KNN saved vector schema is empty",
		));
	}
	let mut source_indices = BTreeSet::new();
	let mut names = BTreeSet::new();
	let mut previous = None;
	for (index, vector) in references.vectors.iter().enumerate() {
		if !source_indices.insert(vector.source_index()) {
			return Err(CheckpointError::manifest(format!(
				"KNN vector source index {} appears more than once",
				vector.source_index()
			)));
		}
		if previous.is_some_and(|prior| prior >= vector.source_index()) {
			return Err(CheckpointError::manifest(
				"KNN vector schemas must retain strictly increasing source order",
			));
		}
		previous = Some(vector.source_index());
		if vector.name().is_empty() || !names.insert(vector.name()) {
			return Err(CheckpointError::manifest(
				"KNN vector names must be nonempty and unique",
			));
		}
		validate_saved_vector(
			vector,
			&CheckpointPath::root().field("vectors").index(index),
		)?;
	}
	Ok(())
}

fn validate_spans(references: &KnnReferenceSet) -> CheckpointResult<()> {
	if references.feature_spans.is_empty() {
		return Err(CheckpointError::manifest("KNN feature span list is empty"));
	}
	let feature_sources = references
		.vectors
		.iter()
		.filter(|vector| vector.role() == VectorRole::Feature)
		.map(CheckpointArtifactVector::source_index)
		.collect::<BTreeSet<_>>();
	let mut seen = BTreeSet::new();
	let mut cursor = 0usize;
	for span in &references.feature_spans {
		if span.start() != cursor || span.width() == 0 {
			return Err(CheckpointError::manifest(
				"KNN feature spans must be nonempty and contiguous from zero",
			));
		}
		if !feature_sources.contains(&span.source_vector()) || !seen.insert(span.source_vector()) {
			return Err(CheckpointError::manifest(
				"KNN feature span source is absent, duplicated, or not a feature",
			));
		}
		match span.lowering() {
			DenseFeatureLowering::NumericScalar if span.width() == 1 => {}
			DenseFeatureLowering::CategoricalOneHot {
				dictionary_width,
				reserved_index,
			} if reserved_index == dictionary_width && span.width() == dictionary_width.saturating_add(1) => {}
			_ => {
				return Err(CheckpointError::manifest(
					"KNN feature span lowering is internally inconsistent",
				));
			}
		}
		cursor = cursor
			.checked_add(span.width())
			.ok_or_else(|| CheckpointError::manifest("KNN feature span width overflowed usize"))?;
	}
	if cursor != references.feature_width || seen != feature_sources {
		return Err(CheckpointError::manifest(
			"KNN feature spans do not exactly cover every saved feature and feature column",
		));
	}
	Ok(())
}

fn validate_mask(references: &KnnReferenceSet) -> CheckpointResult<()> {
	if let Some(mask) = &references.normalization_mask {
		if mask.len() != references.feature_width
			|| mask
				.iter()
				.any(|bits| !matches!(*bits, 0x0000_0000 | 0x3f80_0000))
		{
			return Err(CheckpointError::manifest(
				"KNN normalization mask must contain one exact positive-zero/one f32 bit per feature column",
			));
		}
	}
	Ok(())
}

fn validate_outputs(references: &KnnReferenceSet) -> CheckpointResult<()> {
	if references.outputs.is_empty() {
		return Err(CheckpointError::manifest(
			"KNN model has no declared outputs",
		));
	}
	let vectors = references
		.vectors
		.iter()
		.map(|vector| (vector.source_index(), vector))
		.collect::<BTreeMap<_, _>>();
	let mut seen = BTreeSet::new();
	for output in &references.outputs {
		let source = output.schema.source_index();
		if !seen.insert(source)
			|| vectors.get(&source).copied() != Some(&output.schema)
			|| output.schema.role() != VectorRole::Target
		{
			return Err(CheckpointError::manifest(
				"KNN output schemas must be distinct exact target schemas from the saved vector list",
			));
		}
		if output.known.len() != references.reference_rows
			|| output.known.iter().any(|value| !matches!(*value, 0 | 1))
		{
			return Err(CheckpointError::manifest(
				"KNN output known mask is not one binary value per reference row",
			));
		}
		let count = u64::try_from(output.known.iter().filter(|value| **value == 1).count())
			.map_err(|error| CheckpointError::manifest(format!("KNN known count does not fit u64: {error}")))?;
		if count == 0 || count != output.known_references {
			return Err(CheckpointError::manifest(
				"KNN output known-reference count is zero or differs from its mask",
			));
		}
		match &output.values {
			KnnReferenceValues::NumericF32Bits(bits) => {
				if bits.len() != references.reference_rows
					|| bits.iter().any(|bits| !f32::from_bits(*bits).is_finite())
				{
					return Err(CheckpointError::manifest(
						"KNN numeric output must contain one finite f32 per reference row",
					));
				}
			}
			KnnReferenceValues::DiscreteI32 { codes, labels } => {
				if codes.len() != references.reference_rows
					|| labels.is_empty() || labels.len() > i32::MAX as usize
				{
					return Err(CheckpointError::manifest(
						"KNN discrete output has an invalid row or label count",
					));
				}
				let mut unique = BTreeSet::new();
				if labels.iter().any(|label| !unique.insert(label)) {
					return Err(CheckpointError::manifest(
						"KNN discrete output labels are not unique",
					));
				}
				if codes
					.iter()
					.any(|code| usize::try_from(*code).map_or(true, |index| index >= labels.len()))
				{
					return Err(CheckpointError::manifest(
						"KNN discrete output contains an invalid class code",
					));
				}
			}
		}
	}
	Ok(())
}

#[derive(Debug)]
struct Decoder {
	graph: Graph,
	limits: KnnModelDecodeLimits,
	payload_bytes: usize,
	metadata_entries: usize,
	labels: usize,
}

impl Decoder {
	fn new(source: &[u8], limits: KnnModelDecodeLimits) -> CheckpointResult<Self> {
		let root = CheckpointPath::root();
		if source.len() > limits.source_bytes {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				root,
				format!(
					"KNN model source has {} bytes, limit is {}",
					source.len(),
					limits.source_bytes
				),
			));
		}
		let node_bound = if source.is_empty() {
			0
		} else {
			1usize.checked_add(
				source.iter()
					.filter(|byte| matches!(**byte, b'\n' | b'\t'))
					.count(),
			)
			.ok_or_else(|| {
				decode_error(
					CheckpointDecodeErrorKind::LimitExceeded,
					CheckpointPath::root(),
					"KNN model node pre-count overflowed usize",
				)
			})?
		};
		if node_bound > limits.nodes {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				CheckpointPath::root(),
				format!(
					"KNN model has at least {node_bound} nodes, limit is {}",
					limits.nodes
				),
			));
		}
		let text = core::str::from_utf8(source).map_err(|error| {
			decode_error(
				CheckpointDecodeErrorKind::InvalidUtf8,
				CheckpointPath::root(),
				format!("KNN model is not UTF-8: {error}"),
			)
		})?;
		let graph = Graph::parse(text).map_err(|error| {
			decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax,
				CheckpointPath::root(),
				format!("invalid KNN OGDL: {error}"),
			)
		})?;
		if graph.len() > limits.nodes {
			return Err(decode_error(
				CheckpointDecodeErrorKind::LimitExceeded,
				CheckpointPath::root(),
				format!(
					"KNN model has {} nodes, limit is {}",
					graph.len(),
					limits.nodes
				),
			));
		}
		Ok(Self {
			graph,
			limits,
			payload_bytes: 0,
			metadata_entries: 0,
			labels: 0,
		})
	}

	fn decode(mut self) -> CheckpointResult<KnnModelArtifact> {
		let path = CheckpointPath::root();
		let [root] = self.graph.roots() else {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax,
				path,
				format!(
					"KNN model requires exactly one root, found {}",
					self.graph.roots().len()
				),
			));
		};
		if self.node(*root, &CheckpointPath::root())?.text() != ROOT {
			return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidValue,
				CheckpointPath::root(),
				format!("KNN model root must be {ROOT:?}"),
			));
		}
		let fields = self.fields(*root, &CheckpointPath::root(), &[
			"format-version",
			"neighbors",
			"data-normalization",
			"operations",
			"vectors",
			"feature-spans",
			"normalization-mask",
			"reference-shape",
			"reference-source-rows",
			"reference-feature-f32-bits",
			"outputs",
		])?;
		let format_version = self.parse_u32(
			self.scalar(
				fields["format-version"],
				&CheckpointPath::root().field("format-version"),
			)?,
			&CheckpointPath::root().field("format-version"),
		)?;
		if format_version != KNN_MODEL_FORMAT_VERSION {
			return Err(self.invalid_value(
				&CheckpointPath::root().field("format-version"),
				format!("unsupported KNN model version {format_version}"),
			));
		}
		let neighbors = self.parse_u64(
			self.scalar(
				fields["neighbors"],
				&CheckpointPath::root().field("neighbors"),
			)?,
			&CheckpointPath::root().field("neighbors"),
		)?;
		let neighbors = NonZeroU64::new(neighbors).ok_or_else(|| {
			self.invalid_value(
				&CheckpointPath::root().field("neighbors"),
				"neighbor count must be nonzero",
			)
		})?;
		let data_normalization = self.parse_data_normalization(
			self.scalar(
				fields["data-normalization"],
				&CheckpointPath::root().field("data-normalization"),
			)?,
			&CheckpointPath::root().field("data-normalization"),
		)?;
		let operations = self.parse_operations(
			fields["operations"],
			&CheckpointPath::root().field("operations"),
		)?;
		let vectors = self.parse_vectors(fields["vectors"], &CheckpointPath::root().field("vectors"))?;
		let feature_spans = self.parse_spans(
			fields["feature-spans"],
			&CheckpointPath::root().field("feature-spans"),
		)?;
		let normalization_mask = self.parse_mask(
			fields["normalization-mask"],
			&CheckpointPath::root().field("normalization-mask"),
		)?;
		let shape = self.fields(
			fields["reference-shape"],
			&CheckpointPath::root().field("reference-shape"),
			&["rows", "feature-width"],
		)?;
		let reference_rows = self.parse_usize(
			self.scalar(
				shape["rows"],
				&CheckpointPath::root()
					.field("reference-shape")
					.field("rows"),
			)?,
			&CheckpointPath::root()
				.field("reference-shape")
				.field("rows"),
		)?;
		let feature_width = self.parse_usize(
			self.scalar(
				shape["feature-width"],
				&CheckpointPath::root()
					.field("reference-shape")
					.field("feature-width"),
			)?,
			&CheckpointPath::root()
				.field("reference-shape")
				.field("feature-width"),
		)?;
		if reference_rows == 0 || reference_rows > self.limits.reference_rows {
			return Err(self.limit(
				&CheckpointPath::root()
					.field("reference-shape")
					.field("rows"),
				format!(
					"reference row count is {reference_rows}, limit is {}",
					self.limits.reference_rows
				),
			));
		}
		if feature_width == 0 || feature_width > self.limits.feature_width {
			return Err(self.limit(
				&CheckpointPath::root()
					.field("reference-shape")
					.field("feature-width"),
				format!(
					"feature width is {feature_width}, limit is {}",
					self.limits.feature_width
				),
			));
		}
		let reference_source_rows_value = self
			.scalar(
				fields["reference-source-rows"],
				&CheckpointPath::root().field("reference-source-rows"),
			)?
			.to_owned();
		let reference_source_rows = self.parse_usize_hex(
			&reference_source_rows_value,
			reference_rows,
			&CheckpointPath::root().field("reference-source-rows"),
		)?;
		let feature_elements = reference_rows.checked_mul(feature_width).ok_or_else(|| {
			self.limit(
				&CheckpointPath::root().field("reference-shape"),
				"reference feature element count overflowed usize",
			)
		})?;
		let reference_feature_bits_value = self
			.scalar(
				fields["reference-feature-f32-bits"],
				&CheckpointPath::root().field("reference-feature-f32-bits"),
			)?
			.to_owned();
		let reference_feature_bits = self.parse_u32_hex(
			&reference_feature_bits_value,
			feature_elements,
			&CheckpointPath::root().field("reference-feature-f32-bits"),
		)?;
		let outputs = self.parse_outputs(
			fields["outputs"],
			&CheckpointPath::root().field("outputs"),
			&vectors,
			reference_rows,
		)?;
		let artifact = KnnModelArtifact {
			format_version,
			references: KnnReferenceSet {
				neighbors,
				vectors,
				feature_spans,
				normalization_mask,
				reference_source_rows,
				reference_rows,
				feature_width,
				reference_feature_bits,
				outputs,
			},
			data_normalization,
			operations,
		};
		validate_artifact(&artifact).map_err(|error| {
			match error {
				CheckpointError::Decode(error) => CheckpointError::Decode(error),
				other => {
					decode_error(
						CheckpointDecodeErrorKind::InconsistentValue,
						CheckpointPath::root(),
						other.to_string(),
					)
				}
			}
		})?;
		Ok(artifact)
	}

	fn parse_operations(&self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<Vec<DenseOperation>> {
		let children = self.node(node, path)?.children();
		let mut operations = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "operation" {
				return Err(self.unknown(&item_path, "expected operation entry"));
			}
			operations.push(self.parse_operation(self.scalar(child, &item_path)?, &item_path)?);
		}
		Ok(operations)
	}

	fn parse_vectors(
		&mut self,
		node: NodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointArtifactVector>> {
		let children = self.node(node, path)?.children().to_vec();
		if children.len() > self.limits.vectors {
			return Err(self.limit(
				path,
				format!(
					"vector count is {}, limit is {}",
					children.len(),
					self.limits.vectors
				),
			));
		}
		let mut vectors = Vec::with_capacity(children.len());
		for (index, child) in children.into_iter().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "vector" {
				return Err(self.unknown(&item_path, "expected vector entry"));
			}
			let fields = self.fields(child, &item_path, &[
				"source-index",
				"name-bytes",
				"role",
				"semantic-type",
				"encoding",
				"metadata",
			])?;
			let source_index = self.parse_usize(
				self.scalar(fields["source-index"], &item_path.field("source-index"))?,
				&item_path.field("source-index"),
			)?;
			let name_value = self
				.scalar(fields["name-bytes"], &item_path.field("name-bytes"))?
				.to_owned();
			let name = self.parse_bytes(&name_value, &item_path.field("name-bytes"))?;
			let role = self.parse_role(
				self.scalar(fields["role"], &item_path.field("role"))?,
				&item_path.field("role"),
			)?;
			let semantic_type = self.parse_semantic(
				self.scalar(fields["semantic-type"], &item_path.field("semantic-type"))?,
				&item_path.field("semantic-type"),
			)?;
			let encoding = self.parse_encoding(
				self.scalar(fields["encoding"], &item_path.field("encoding"))?,
				&item_path.field("encoding"),
			)?;
			let metadata = self.parse_metadata(fields["metadata"], &item_path.field("metadata"))?;
			vectors.push(CheckpointArtifactVector::new(
				source_index,
				name,
				role,
				semantic_type,
				encoding,
				metadata,
			));
		}
		Ok(vectors)
	}

	fn parse_metadata(
		&mut self,
		node: NodeId,
		path: &CheckpointPath,
	) -> CheckpointResult<CheckpointArtifactMetadata> {
		let tag = self.tag(node, path)?;
		let text = self.node(tag, path)?.text().to_owned();
		let children = self.node(tag, path)?.children().to_vec();
		match text.as_str() {
			"none" => {
				self.require_empty(&children, path)?;
				Ok(CheckpointArtifactMetadata::None)
			}
			"temporal" => {
				let fields = self.fields_from(&children, path, &["unix-seconds", "nanoseconds"])?;
				let unix_seconds = self.parse_i64(
					self.scalar(fields["unix-seconds"], &path.field("unix-seconds"))?,
					&path.field("unix-seconds"),
				)?;
				let nanoseconds = self.parse_u32(
					self.scalar(fields["nanoseconds"], &path.field("nanoseconds"))?,
					&path.field("nanoseconds"),
				)?;
				Ok(CheckpointArtifactMetadata::Temporal {
					unix_seconds,
					nanoseconds,
				})
			}
			"categorical" => {
				Ok(CheckpointArtifactMetadata::Categorical {
					dictionary: self.parse_byte_entries(&children, path)?,
				})
			}
			"ordinal" => {
				Ok(CheckpointArtifactMetadata::Ordinal {
					ordered_labels: self.parse_byte_entries(&children, path)?,
				})
			}
			"image" => {
				Ok(CheckpointArtifactMetadata::Image {
					encoded_variants: self.parse_image_entries(&children, path)?,
				})
			}
			_ => Err(self.invalid_value(path, format!("unknown vector metadata {text:?}"))),
		}
	}

	fn parse_byte_entries(&mut self, children: &[NodeId], path: &CheckpointPath) -> CheckpointResult<Vec<Vec<u8>>> {
		self.reserve_metadata(children.len(), path)?;
		let mut values = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "value-bytes" {
				return Err(self.unknown(&item_path, "expected value-bytes metadata entry"));
			}
			let value = self.scalar(child, &item_path)?.to_owned();
			values.push(self.parse_bytes(&value, &item_path)?);
		}
		Ok(values)
	}

	fn parse_image_entries(
		&mut self,
		children: &[NodeId],
		path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointImageMetadata>> {
		self.reserve_metadata(children.len(), path)?;
		let mut values = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "variant" {
				return Err(self.unknown(&item_path, "expected image variant"));
			}
			let fields = self.fields(child, &item_path, &[
				"format",
				"width",
				"height",
				"channels",
				"color-model",
				"sample-bits",
				"value-layout",
				"value-range",
			])?;
			let format = self.parse_image_format(
				self.scalar(fields["format"], &item_path.field("format"))?,
				&item_path.field("format"),
			)?;
			let width = self.parse_u32(
				self.scalar(fields["width"], &item_path.field("width"))?,
				&item_path.field("width"),
			)?;
			let height = self.parse_u32(
				self.scalar(fields["height"], &item_path.field("height"))?,
				&item_path.field("height"),
			)?;
			let channels = self.parse_optional_u8(
				self.scalar(fields["channels"], &item_path.field("channels"))?,
				&item_path.field("channels"),
			)?;
			let color_model = self.parse_color_model(
				self.scalar(fields["color-model"], &item_path.field("color-model"))?,
				&item_path.field("color-model"),
			)?;
			let sample_bits = self.parse_optional_u8(
				self.scalar(fields["sample-bits"], &item_path.field("sample-bits"))?,
				&item_path.field("sample-bits"),
			)?;
			self.expect(
				self.scalar(fields["value-layout"], &item_path.field("value-layout"))?,
				"encoded-file",
				&item_path.field("value-layout"),
			)?;
			self.expect(
				self.scalar(fields["value-range"], &item_path.field("value-range"))?,
				"encoded-bytes",
				&item_path.field("value-range"),
			)?;
			values.push(CheckpointImageMetadata::new(
				format,
				width,
				height,
				channels,
				color_model,
				sample_bits,
				ImageValueLayout::EncodedFile,
				ImageValueRange::EncodedBytes,
			));
		}
		Ok(values)
	}

	fn parse_spans(&self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<Vec<CompiledFeatureSpan>> {
		let children = self.node(node, path)?.children();
		if children.len() > self.limits.feature_spans {
			return Err(self.limit(
				path,
				format!(
					"feature-span count is {}, limit is {}",
					children.len(),
					self.limits.feature_spans
				),
			));
		}
		let mut spans = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "span" {
				return Err(self.unknown(&item_path, "expected feature span"));
			}
			let fields = self.fields(child, &item_path, &[
				"source-index",
				"start",
				"width",
				"lowering",
			])?;
			let source_index = self.parse_usize(
				self.scalar(fields["source-index"], &item_path.field("source-index"))?,
				&item_path.field("source-index"),
			)?;
			let start = self.parse_usize(
				self.scalar(fields["start"], &item_path.field("start"))?,
				&item_path.field("start"),
			)?;
			let width = self.parse_usize(
				self.scalar(fields["width"], &item_path.field("width"))?,
				&item_path.field("width"),
			)?;
			let lowering_tag = self.tag(fields["lowering"], &item_path.field("lowering"))?;
			let lowering_name = self
				.node(lowering_tag, &item_path.field("lowering"))?
				.text();
			let lowering = match lowering_name {
				"numeric-scalar" => {
					self.require_empty(
						self.node(lowering_tag, &item_path)?.children(),
						&item_path.field("lowering"),
					)?;
					DenseFeatureLowering::NumericScalar
				}
				"categorical-one-hot" => {
					let tagged_fields = self.fields(lowering_tag, &item_path.field("lowering"), &[
						"dictionary-width",
						"reserved-index",
					])?;
					DenseFeatureLowering::CategoricalOneHot {
						dictionary_width: self.parse_usize(
							self.scalar(
								tagged_fields["dictionary-width"],
								&item_path.field("lowering").field("dictionary-width"),
							)?,
							&item_path.field("lowering").field("dictionary-width"),
						)?,
						reserved_index: self.parse_usize(
							self.scalar(
								tagged_fields["reserved-index"],
								&item_path.field("lowering").field("reserved-index"),
							)?,
							&item_path.field("lowering").field("reserved-index"),
						)?,
					}
				}
				_ => return Err(self.invalid_value(&item_path.field("lowering"), "unknown feature lowering")),
			};
			spans.push(CompiledFeatureSpan::new(
				source_index,
				start,
				width,
				lowering,
			));
		}
		Ok(spans)
	}

	fn parse_mask(&mut self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<Option<Vec<u32>>> {
		let tag = self.tag(node, path)?;
		match self.node(tag, path)?.text() {
			"none" => {
				self.require_empty(self.node(tag, path)?.children(), path)?;
				Ok(None)
			}
			"f32-bits" => {
				let scalar = self.scalar(tag, path)?.to_owned();
				let count = scalar
					.strip_prefix("0x")
					.map_or(0, |digits| digits.len() / 8);
				self.parse_u32_hex(&scalar, count, path).map(Some)
			}
			_ => Err(self.invalid_value(path, "unknown normalization-mask tag")),
		}
	}

	fn parse_outputs(
		&mut self,
		node: NodeId,
		path: &CheckpointPath,
		vectors: &[CheckpointArtifactVector],
		reference_rows: usize,
	) -> CheckpointResult<Vec<KnnReferenceOutput>> {
		let children = self.node(node, path)?.children().to_vec();
		if children.is_empty() || children.len() > self.limits.outputs {
			return Err(self.limit(
				path,
				format!(
					"output count is {}, limit is {}",
					children.len(),
					self.limits.outputs
				),
			));
		}
		let schemas = vectors
			.iter()
			.filter(|vector| vector.role() == VectorRole::Target)
			.map(|vector| (vector.source_index(), vector))
			.collect::<BTreeMap<_, _>>();
		let mut outputs = Vec::with_capacity(children.len());
		for (index, child) in children.into_iter().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "output" {
				return Err(self.unknown(&item_path, "expected output entry"));
			}
			let fields = self.fields(child, &item_path, &["source-index", "known-mask", "values"])?;
			let source_index = self.parse_usize(
				self.scalar(fields["source-index"], &item_path.field("source-index"))?,
				&item_path.field("source-index"),
			)?;
			let schema = schemas.get(&source_index).copied().ok_or_else(|| {
				self.invalid_value(
					&item_path.field("source-index"),
					"output source is not a saved target vector",
				)
			})?;
			let known_value = self
				.scalar(fields["known-mask"], &item_path.field("known-mask"))?
				.to_owned();
			let known = self.parse_known(&known_value, reference_rows, &item_path.field("known-mask"))?;
			let known_references =
				u64::try_from(known.iter().filter(|value| **value == 1).count()).map_err(|error| {
					self.invalid_value(
						&item_path.field("known-mask"),
						format!("known count does not fit u64: {error}"),
					)
				})?;
			let tag = self.tag(fields["values"], &item_path.field("values"))?;
			let values = match self.node(tag, &item_path.field("values"))?.text() {
				"numeric-f32-bits" => {
					let value = self.scalar(tag, &item_path.field("values"))?.to_owned();
					KnnReferenceValues::NumericF32Bits(self.parse_u32_hex(
						&value,
						reference_rows,
						&item_path.field("values"),
					)?)
				}
				"discrete-int32" => {
					let value_fields = self.fields(tag, &item_path.field("values"), &["codes", "labels"])?;
					let codes_value = self
						.scalar(
							value_fields["codes"],
							&item_path.field("values").field("codes"),
						)?
						.to_owned();
					let codes = self.parse_i32_hex(
						&codes_value,
						reference_rows,
						&item_path.field("values").field("codes"),
					)?;
					let labels = self.parse_labels(
						value_fields["labels"],
						&item_path.field("values").field("labels"),
					)?;
					KnnReferenceValues::DiscreteI32 { codes, labels }
				}
				_ => {
					return Err(
						self.invalid_value(&item_path.field("values"), "unknown KNN output value kind")
					);
				}
			};
			outputs.push(KnnReferenceOutput {
				schema: schema.clone(),
				known,
				known_references,
				values,
			});
		}
		Ok(outputs)
	}

	fn parse_labels(&mut self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<Vec<KnnLabelValue>> {
		let children = self.node(node, path)?.children().to_vec();
		self.labels = self
			.labels
			.checked_add(children.len())
			.ok_or_else(|| self.limit(path, "label count overflowed"))?;
		if self.labels > self.limits.labels {
			return Err(self.limit(
				path,
				format!(
					"label count is {}, limit is {}",
					self.labels, self.limits.labels
				),
			));
		}
		let mut labels = Vec::with_capacity(children.len());
		for (index, child) in children.into_iter().enumerate() {
			let item_path = path.index(index);
			let kind = self.node(child, &item_path)?.text().to_owned();
			let value = self.scalar(child, &item_path)?.to_owned();
			labels.push(match kind.as_str() {
				"int32" => KnnLabelValue::I32(self.parse_i32(&value, &item_path)?),
				"bytes" => KnnLabelValue::Bytes(self.parse_bytes(&value, &item_path)?),
				_ => return Err(self.unknown(&item_path, "expected int32 or bytes label")),
			});
		}
		Ok(labels)
	}

	fn fields(
		&self,
		node: NodeId,
		path: &CheckpointPath,
		required: &[&str],
	) -> CheckpointResult<BTreeMap<String, NodeId>> {
		self.fields_from(self.node(node, path)?.children(), path, required)
	}

	fn fields_from(
		&self,
		children: &[NodeId],
		path: &CheckpointPath,
		required: &[&str],
	) -> CheckpointResult<BTreeMap<String, NodeId>> {
		let allowed = required.iter().copied().collect::<BTreeSet<_>>();
		let mut fields = BTreeMap::new();
		for child in children {
			let name = self.node(*child, path)?.text();
			if !allowed.contains(name) {
				return Err(self.unknown(&path.field(name), format!("unknown field {name:?}")));
			}
			if fields.insert(name.to_owned(), *child).is_some() {
				return Err(decode_error(
					CheckpointDecodeErrorKind::DuplicateField,
					path.field(name),
					format!("field {name:?} appears more than once"),
				));
			}
		}
		for name in required {
			if !fields.contains_key(*name) {
				return Err(decode_error(
					CheckpointDecodeErrorKind::MissingField,
					path.field(*name),
					format!("required field {name:?} is absent"),
				));
			}
		}
		Ok(fields)
	}

	fn node(&self, id: NodeId, path: &CheckpointPath) -> CheckpointResult<&recipe_ogdl::Node> {
		self.graph.node(id).ok_or_else(|| {
			decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax,
				path.clone(),
				"KNN OGDL contains an unknown node identity",
			)
		})
	}

	fn scalar<'a>(&'a self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<&'a str> {
		let children = self.node(node, path)?.children();
		let [value] = children else {
			return Err(self.invalid_value(path, format!("scalar field has {} values", children.len())));
		};
		let value = self.node(*value, path)?;
		if !value.children().is_empty() {
			return Err(self.invalid_value(path, "scalar value has descendants"));
		}
		Ok(value.text())
	}

	fn tag(&self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<NodeId> {
		let children = self.node(node, path)?.children();
		let [tag] = children else {
			return Err(self.invalid_value(path, format!("tagged field has {} tags", children.len())));
		};
		Ok(*tag)
	}

	fn require_empty(&self, children: &[NodeId], path: &CheckpointPath) -> CheckpointResult<()> {
		if children.is_empty() {
			Ok(())
		} else {
			Err(self.unknown(path, "tag has unexpected descendants"))
		}
	}

	fn reserve_payload(&mut self, bytes: usize, path: &CheckpointPath) -> CheckpointResult<()> {
		self.payload_bytes = self
			.payload_bytes
			.checked_add(bytes)
			.ok_or_else(|| self.limit(path, "payload byte count overflowed"))?;
		if self.payload_bytes > self.limits.total_payload_bytes {
			return Err(self.limit(
				path,
				format!(
					"decoded payload is {} bytes, limit is {}",
					self.payload_bytes, self.limits.total_payload_bytes
				),
			));
		}
		Ok(())
	}

	fn reserve_metadata(&mut self, entries: usize, path: &CheckpointPath) -> CheckpointResult<()> {
		self.metadata_entries = self
			.metadata_entries
			.checked_add(entries)
			.ok_or_else(|| self.limit(path, "metadata entry count overflowed"))?;
		if self.metadata_entries > self.limits.metadata_entries {
			return Err(self.limit(
				path,
				format!(
					"metadata entry count is {}, limit is {}",
					self.metadata_entries, self.limits.metadata_entries
				),
			));
		}
		Ok(())
	}

	fn parse_bytes(&mut self, value: &str, path: &CheckpointPath) -> CheckpointResult<Vec<u8>> {
		let digits = canonical_hex(value, path)?;
		if digits.len() % 2 != 0 {
			return Err(self.invalid_value(path, "hex byte string has an odd digit count"));
		}
		let bytes = digits.len() / 2;
		self.reserve_payload(bytes, path)?;
		(0..bytes)
			.map(|index| parse_hex_byte(&digits[index * 2..index * 2 + 2], path))
			.collect()
	}

	fn parse_u32_hex(&mut self, value: &str, count: usize, path: &CheckpointPath) -> CheckpointResult<Vec<u32>> {
		let digits = canonical_hex(value, path)?;
		let expected = count
			.checked_mul(8)
			.ok_or_else(|| self.limit(path, "u32 hex digit count overflowed"))?;
		if digits.len() != expected {
			return Err(self.invalid_value(
				path,
				format!("u32 image has {} digits, expected {expected}", digits.len()),
			));
		}
		self.reserve_payload(
			count.checked_mul(4)
				.ok_or_else(|| self.limit(path, "u32 payload overflowed"))?,
			path,
		)?;
		(0..count)
			.map(|index| {
				u32::from_str_radix(&digits[index * 8..index * 8 + 8], 16)
					.map_err(|error| self.invalid_value(path, error.to_string()))
			})
			.collect()
	}

	fn parse_i32_hex(&mut self, value: &str, count: usize, path: &CheckpointPath) -> CheckpointResult<Vec<i32>> {
		self.parse_u32_hex(value, count, path)
			.map(|values| values.into_iter().map(|value| value as i32).collect())
	}

	fn parse_usize_hex(&mut self, value: &str, count: usize, path: &CheckpointPath) -> CheckpointResult<Vec<usize>> {
		let digits = canonical_hex(value, path)?;
		let expected = count
			.checked_mul(16)
			.ok_or_else(|| self.limit(path, "usize hex digit count overflowed"))?;
		if digits.len() != expected {
			return Err(self.invalid_value(
				path,
				format!(
					"source-row image has {} digits, expected {expected}",
					digits.len()
				),
			));
		}
		self.reserve_payload(
			count.checked_mul(8)
				.ok_or_else(|| self.limit(path, "source-row payload overflowed"))?,
			path,
		)?;
		(0..count)
			.map(|index| {
				let value = u64::from_str_radix(&digits[index * 16..index * 16 + 16], 16)
					.map_err(|error| self.invalid_value(path, error.to_string()))?;
				usize::try_from(value).map_err(|error| {
					self.invalid_value(path, format!("source row does not fit usize: {error}"))
				})
			})
			.collect()
	}

	fn parse_known(&mut self, value: &str, count: usize, path: &CheckpointPath) -> CheckpointResult<Vec<i32>> {
		let digits = value
			.strip_prefix("0b")
			.ok_or_else(|| self.invalid_value(path, "known mask lacks canonical 0b prefix"))?;
		if digits.len() != count || digits.bytes().any(|byte| !matches!(byte, b'0' | b'1')) {
			return Err(self.invalid_value(
				path,
				format!("known mask must contain exactly {count} binary digits"),
			));
		}
		self.reserve_payload(count, path)?;
		Ok(digits.bytes().map(|byte| i32::from(byte == b'1')).collect())
	}

	fn parse_usize(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<usize> {
		parse_canonical(value, path)
	}

	fn parse_u64(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<u64> { parse_canonical(value, path) }

	fn parse_u32(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<u32> { parse_canonical(value, path) }

	fn parse_i32(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<i32> { parse_canonical(value, path) }

	fn parse_i64(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<i64> { parse_canonical(value, path) }

	fn parse_optional_u8(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<Option<u8>> {
		if value == "none" {
			Ok(None)
		} else {
			parse_canonical(value, path).map(Some)
		}
	}

	fn parse_data_normalization(
		&self,
		value: &str,
		path: &CheckpointPath,
	) -> CheckpointResult<Option<DenseDataNormalization>> {
		match value {
			"none" => Ok(None),
			"z-score" => Ok(Some(DenseDataNormalization::ZScore)),
			"min-max" => Ok(Some(DenseDataNormalization::MinMax)),
			"l2-norm" => Ok(Some(DenseDataNormalization::L2Norm)),
			_ => Err(self.invalid_value(path, "unknown data normalization")),
		}
	}

	fn parse_operation(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<DenseOperation> {
		parse_operation(value).ok_or_else(|| self.invalid_value(path, format!("unknown dense operation {value:?}")))
	}

	fn parse_role(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<VectorRole> {
		match value {
			"feature" => Ok(VectorRole::Feature),
			"target" => Ok(VectorRole::Target),
			_ => Err(self.invalid_value(path, "unknown vector role")),
		}
	}

	fn parse_semantic(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<SemanticType> {
		parse_semantic_type(value).ok_or_else(|| self.invalid_value(path, "unknown semantic type"))
	}

	fn parse_encoding(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<VectorEncoding> {
		parse_vector_encoding(value).ok_or_else(|| self.invalid_value(path, "unknown vector encoding"))
	}

	fn parse_image_format(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<EncodedImageFormat> {
		parse_image_format(value).ok_or_else(|| self.invalid_value(path, "unknown image format"))
	}

	fn parse_color_model(&self, value: &str, path: &CheckpointPath) -> CheckpointResult<Option<ImageColorModel>> {
		if value == "none" {
			Ok(None)
		} else {
			parse_image_color_model(value)
				.map(Some)
				.ok_or_else(|| self.invalid_value(path, "unknown image color model"))
		}
	}

	fn expect(&self, actual: &str, expected: &str, path: &CheckpointPath) -> CheckpointResult<()> {
		if actual == expected {
			Ok(())
		} else {
			Err(self.invalid_value(path, format!("expected {expected:?}, found {actual:?}")))
		}
	}

	fn invalid_value(&self, path: &CheckpointPath, detail: impl Into<String>) -> CheckpointError {
		decode_error(
			CheckpointDecodeErrorKind::InvalidValue,
			path.clone(),
			detail,
		)
	}

	fn unknown(&self, path: &CheckpointPath, detail: impl Into<String>) -> CheckpointError {
		decode_error(
			CheckpointDecodeErrorKind::UnknownField,
			path.clone(),
			detail,
		)
	}

	fn limit(&self, path: &CheckpointPath, detail: impl Into<String>) -> CheckpointError {
		decode_error(
			CheckpointDecodeErrorKind::LimitExceeded,
			path.clone(),
			detail,
		)
	}
}

fn canonical_hex<'a>(value: &'a str, path: &CheckpointPath) -> CheckpointResult<&'a str> {
	let digits = value.strip_prefix("0x").ok_or_else(|| {
		decode_error(
			CheckpointDecodeErrorKind::InvalidValue,
			path.clone(),
			"hex value lacks canonical lowercase 0x prefix",
		)
	})?;
	if digits
		.bytes()
		.any(|byte| !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
	{
		return Err(decode_error(
			CheckpointDecodeErrorKind::InvalidValue,
			path.clone(),
			"hex value contains a noncanonical digit",
		));
	}
	Ok(digits)
}

fn parse_hex_byte(value: &str, path: &CheckpointPath) -> CheckpointResult<u8> {
	u8::from_str_radix(value, 16).map_err(|error| {
		decode_error(
			CheckpointDecodeErrorKind::InvalidValue,
			path.clone(),
			format!("invalid hex byte: {error}"),
		)
	})
}

fn parse_canonical<T>(value: &str, path: &CheckpointPath) -> CheckpointResult<T>
where
	T: core::str::FromStr + ToString,
	T::Err: core::fmt::Display,
{
	let parsed = value.parse::<T>().map_err(|error| {
		decode_error(
			CheckpointDecodeErrorKind::InvalidValue,
			path.clone(),
			format!("invalid integer {value:?}: {error}"),
		)
	})?;
	if parsed.to_string() != value {
		return Err(decode_error(
			CheckpointDecodeErrorKind::InvalidValue,
			path.clone(),
			format!("integer {value:?} is not canonical"),
		));
	}
	Ok(parsed)
}

const fn vector_role(role: VectorRole) -> &'static str {
	match role {
		VectorRole::Feature => "feature",
		VectorRole::Target => "target",
	}
}

const fn semantic_type(value: SemanticType) -> &'static str {
	match value {
		SemanticType::Numeric => "numeric",
		SemanticType::Temporal => "temporal",
		SemanticType::Categorical => "categorical",
		SemanticType::Ordinal => "ordinal",
		SemanticType::Text => "text",
		SemanticType::Image => "image",
		SemanticType::Binary => "binary",
	}
}

fn parse_semantic_type(value: &str) -> Option<SemanticType> {
	match value {
		"numeric" => Some(SemanticType::Numeric),
		"temporal" => Some(SemanticType::Temporal),
		"categorical" => Some(SemanticType::Categorical),
		"ordinal" => Some(SemanticType::Ordinal),
		"text" => Some(SemanticType::Text),
		"image" => Some(SemanticType::Image),
		"binary" => Some(SemanticType::Binary),
		_ => None,
	}
}

const fn vector_encoding(value: VectorEncoding) -> &'static str {
	match value {
		VectorEncoding::F32 => "f32",
		VectorEncoding::I32 => "int32",
		VectorEncoding::RelativeSecondsI32 => "relative-seconds-int32",
		VectorEncoding::DictionaryI32 => "dictionary-int32",
		VectorEncoding::OrdinalI32 => "ordinal-int32",
		VectorEncoding::Utf8 => "utf8",
		VectorEncoding::Bytes => "bytes",
	}
}

fn parse_vector_encoding(value: &str) -> Option<VectorEncoding> {
	match value {
		"f32" => Some(VectorEncoding::F32),
		"int32" => Some(VectorEncoding::I32),
		"relative-seconds-int32" => Some(VectorEncoding::RelativeSecondsI32),
		"dictionary-int32" => Some(VectorEncoding::DictionaryI32),
		"ordinal-int32" => Some(VectorEncoding::OrdinalI32),
		"utf8" => Some(VectorEncoding::Utf8),
		"bytes" => Some(VectorEncoding::Bytes),
		_ => None,
	}
}

const fn data_normalization(value: DenseDataNormalization) -> &'static str {
	match value {
		DenseDataNormalization::Identity => "identity",
		DenseDataNormalization::ZScore => "z-score",
		DenseDataNormalization::MinMax => "min-max",
		DenseDataNormalization::L2Norm => "l2-norm",
	}
}

const fn dense_operation(value: DenseOperation) -> &'static str { value.token() }

fn parse_operation(value: &str) -> Option<DenseOperation> { DenseOperation::from_token(value) }

const fn image_format(value: EncodedImageFormat) -> &'static str {
	match value {
		EncodedImageFormat::Png => "png",
		EncodedImageFormat::Jpeg => "jpeg",
		EncodedImageFormat::Gif87a => "gif87a",
		EncodedImageFormat::Gif89a => "gif89a",
		EncodedImageFormat::Bmp => "bmp",
		EncodedImageFormat::WebP => "webp",
	}
}

fn parse_image_format(value: &str) -> Option<EncodedImageFormat> {
	match value {
		"png" => Some(EncodedImageFormat::Png),
		"jpeg" => Some(EncodedImageFormat::Jpeg),
		"gif87a" => Some(EncodedImageFormat::Gif87a),
		"gif89a" => Some(EncodedImageFormat::Gif89a),
		"bmp" => Some(EncodedImageFormat::Bmp),
		"webp" => Some(EncodedImageFormat::WebP),
		_ => None,
	}
}

const fn image_color_model(value: ImageColorModel) -> &'static str {
	match value {
		ImageColorModel::Grayscale => "grayscale",
		ImageColorModel::GrayscaleAlpha => "grayscale-alpha",
		ImageColorModel::Rgb => "rgb",
		ImageColorModel::Rgba => "rgba",
		ImageColorModel::Bgr => "bgr",
		ImageColorModel::IndexedRgb => "indexed-rgb",
		ImageColorModel::YCbCr => "y-cb-cr",
		ImageColorModel::Cmyk => "cmyk",
		ImageColorModel::Ycck => "ycck",
	}
}

fn parse_image_color_model(value: &str) -> Option<ImageColorModel> {
	match value {
		"grayscale" => Some(ImageColorModel::Grayscale),
		"grayscale-alpha" => Some(ImageColorModel::GrayscaleAlpha),
		"rgb" => Some(ImageColorModel::Rgb),
		"rgba" => Some(ImageColorModel::Rgba),
		"bgr" => Some(ImageColorModel::Bgr),
		"indexed-rgb" => Some(ImageColorModel::IndexedRgb),
		"y-cb-cr" => Some(ImageColorModel::YCbCr),
		"cmyk" => Some(ImageColorModel::Cmyk),
		"ycck" => Some(ImageColorModel::Ycck),
		_ => None,
	}
}

const fn image_value_layout(value: ImageValueLayout) -> &'static str {
	match value {
		ImageValueLayout::EncodedFile => "encoded-file",
	}
}

const fn image_value_range(value: ImageValueRange) -> &'static str {
	match value {
		ImageValueRange::EncodedBytes => "encoded-bytes",
	}
}
