use alloc::collections::{BTreeMap, BTreeSet}; use core::{fmt, num::NonZeroU64, str};
use std::{io::Write as _, path::Path};

use recipe_ingest::{
	EncodedImageFormat, ImageColorModel, ImageValueLayout, ImageValueRange, SemanticType, VectorEncoding, VectorRole, };
use recipe_ogdl::{Graph, NodeId};

use crate::{ CheckpointArtifactMetadata, CheckpointArtifactVector, CheckpointDecodeErrorKind, CheckpointError,
	CheckpointImageMetadata, CheckpointPath, CheckpointResult, CompiledFeatureSpan, DataNormalization,
	DenseFeatureLowering, Function, KnnLabelValue, KnnReferenceOutput, KnnReferenceSet, KnnReferenceValues,
	checkpoint::{atomic_save, decode_error, validate_saved_vector}, };

/// Canonical semantic KNN artifact format version.
const KNN_MODEL_FORMAT_VERSION: u32 = 1;
/// Canonical OGDL root name for a semantic KNN artifact.
const ROOT: &str = "recipe-knn-model";

/// Finite allocation and structural bounds for one semantic KNN model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KnnModelDecodeLimits { pub source_bytes: usize, pub nodes: usize, pub vectors: usize,
	pub feature_spans: usize, pub reference_rows: usize, pub feature_width: usize, pub outputs: usize, pub labels: usize,
	pub metadata_entries: usize, pub total_payload_bytes: usize, }

impl Default for KnnModelDecodeLimits {
	#[inline]
	fn default() -> Self { return Self { source_bytes: 1 << 30, nodes: 4_000_000, vectors: 0x0001_0000,
			feature_spans: 0x0001_0000, reference_rows: 100_000_000, feature_width: 1_000_000, outputs: 0x0001_0000,
			labels: 1_000_000, metadata_entries: 1_000_000, total_payload_bytes: 1 << 30, }; } }

/// Complete semantic artifact for one all-output KNN model.
///
/// The reference set contains no opaque implementation state: it retains the
/// exact prepared feature image, every independently masked target, and every
/// decoder required to recover nonnumeric predictions. Normalization and
/// post-reduction declarations are stored even though their public execution
/// policy is deliberately not guessed here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KnnModelArtifact {
	/// Semantic artifact format version.
	format_version: u32,
	/// Immutable KNN reference state.
	references: KnnReferenceSet,
	/// Declared feature normalization, when present.
	data_normalization: Option<DataNormalization>,
	/// Ordered post-reduction operations.
	operations: Vec<Function>, }

impl KnnModelArtifact {
	/// Construct and validate one semantic KNN artifact.
	///
	/// # Errors
	///
	/// Returns an error when the supplied reference state or declarations are
	/// inconsistent.
	#[inline]
	pub fn new( references: KnnReferenceSet, data_normalization: Option<DataNormalization>,
		operations: impl IntoIterator<Item = Function>, ) -> CheckpointResult<Self> { let artifact = Self {
			format_version: KNN_MODEL_FORMAT_VERSION, references, data_normalization,
			operations: operations.into_iter().collect(), }; validate_artifact(&artifact)?; return Ok(artifact); }

	#[must_use]
	#[inline]
	pub const fn format_version(&self) -> u32 { return self.format_version; }

	#[must_use]
	#[inline]
	pub const fn references(&self) -> &KnnReferenceSet { return &self.references; }

	#[must_use]
	#[inline]
	pub const fn data_normalization(&self) -> Option<DataNormalization> { return self.data_normalization; }

	#[must_use]
	#[inline]
	pub fn operations(&self) -> &[Function] { return &self.operations; }

	/// Continue one KNN model with another prepared training partition.
	///
	/// The saved references remain first and the newly prepared references are
	/// appended in their retained row order. Repeated observations are retained:
	/// KNN has no global row identity with which to deduplicate them, and their
	/// multiplicity is part of the declared data. This order is also the stable
	/// distance-tie order. The current declaration must retain the saved
	/// neighbors, row-free vector schema, feature lowering, normalization, and
	/// post-reduction topology.
	///
	/// # Errors
	///
	/// Returns an error when either artifact is invalid, their declarations are
	/// incompatible, or their reference storage cannot be combined.
	#[inline]
	pub fn continue_with(mut self, current: Self) -> CheckpointResult<Self> { validate_artifact(&self)?;
		validate_artifact(&current)?; let saved_references = &self.references; let current_references = &current.references;
		if self.format_version != current.format_version || saved_references.neighbors != current_references.neighbors
			|| self.data_normalization != current.data_normalization
			|| self.operations != current.operations
		{ return Err(incompatible_resume(
				"saved and current KNN neighbor count, normalization, or topology differ",
			)); }
		if saved_references.vectors != current_references.vectors
			|| saved_references.feature_spans != current_references.feature_spans
			|| saved_references.normalization_mask != current_references.normalization_mask
			|| saved_references.feature_width != current_references.feature_width
		{ return Err(incompatible_resume(
				"saved and current KNN row-free vector schemas or feature lowering differ",
			)); }
		if saved_references.outputs.len() != current_references.outputs.len() || saved_references .outputs .iter()
				.zip(&current_references.outputs)
				.any(|(saved_output, current_output)| return saved_output.schema != current_output.schema)
		{ return Err(incompatible_resume(
				"saved and current KNN declared output schemas or order differ",
			)); }
		let appended_references = current.references; let combined_rows = self .references .reference_rows
			.checked_add(appended_references.reference_rows)
			.ok_or_else(|| return incompatible_resume("combined KNN reference row count overflowed usize"))?;
		let combined_feature_elements = combined_rows .checked_mul(self.references.feature_width)
			.ok_or_else(|| return incompatible_resume("combined KNN reference feature shape overflowed usize"))?;
		self.references .reference_source_rows .try_reserve_exact(appended_references.reference_source_rows.len())
			.map_err(|error| return incompatible_resume(format!("reserve combined KNN source rows: {error}")))?;
		self.references .reference_feature_bits .try_reserve_exact(appended_references.reference_feature_bits.len())
			.map_err(|error| {
				return incompatible_resume(format!("reserve combined KNN feature image: {error}"));
			})?; self.references .reference_source_rows .extend(appended_references.reference_source_rows); self.references
			.reference_feature_bits .extend(appended_references.reference_feature_bits);
		for (saved_output, current_output) in self .references .outputs .iter_mut() .zip(appended_references.outputs)
		{ let combined_known = saved_output .known_references .checked_add(current_output.known_references) .ok_or_else(|| {
					return incompatible_resume("combined KNN known-reference count overflowed u64");
				})?; saved_output .known .try_reserve_exact(current_output.known.len()) .map_err(|error| {
					return incompatible_resume(format!("reserve combined KNN known mask: {error}"));
				})?; saved_output.known.extend(current_output.known); saved_output.known_references = combined_known;
			let current_values = current_output.values; if saved_output.values.is_numeric() != current_values.is_numeric() {
				return Err(incompatible_resume(
					"saved and current KNN output aggregation kinds differ",
				)); }
			if saved_output.values.is_numeric() { let saved_bits = saved_output.values.numeric_f32_bits_mut();
					let current_bits = current_values.into_numeric_f32_bits(); saved_bits .try_reserve_exact(current_bits.len())
						.map_err(|error| { return incompatible_resume(format!(
								"reserve combined KNN numeric output: {error}"
							)); })?; saved_bits.extend(current_bits); } else {
					let (saved_codes, saved_labels) = saved_output.values.discrete_i32_mut();
					let (current_codes, current_labels) = current_values.into_discrete_i32(); let mut codes_by_label = saved_labels
						.iter() .cloned() .enumerate() .map(|(index, label)| { let code = i32::try_from(index).map_err(|error| {
								return incompatible_resume(format!(
									"saved KNN label index does not fit int32: {error}"
								)); })?; return Ok((label, code)); }) .collect::<CheckpointResult<BTreeMap<_, _>>>()?;
					for label in &current_labels { if codes_by_label.contains_key(label) { continue; }
						let code = i32::try_from(saved_labels.len()).map_err(|error| { return incompatible_resume(format!(
								"combined KNN label index does not fit int32: {error}"
							)); })?; saved_labels.try_reserve_exact(1).map_err(|error| { return incompatible_resume(format!(
								"reserve combined KNN label dictionary: {error}"
							)); })?; saved_labels.push(label.clone()); codes_by_label.insert(label.clone(), code); }
					saved_codes .try_reserve_exact(current_codes.len()) .map_err(|error| { return incompatible_resume(format!(
								"reserve combined KNN discrete output: {error}"
							)); })?; for code in current_codes { let index = usize::try_from(code).map_err(|error| {
							return incompatible_resume(format!(
								"current KNN class code is negative: {error}"
							)); })?; let label = current_labels.get(index).ok_or_else(|| { return incompatible_resume(format!(
								"current KNN class code {code} is outside its dictionary"
							)); })?; let mapped = codes_by_label.get(label).copied().ok_or_else(|| {
							return incompatible_resume("combined KNN label has no class code");
						})?; saved_codes.push(mapped); } } }
		self.references.reference_rows = combined_rows;
		if self.references.reference_feature_bits.len() != combined_feature_elements { return Err(incompatible_resume(
				"combined KNN reference feature image differs from its checked shape",
			)); }
		validate_artifact(&self)?; return Ok(self); }

	/// Encode one strict canonical textual OGDL model image.
	///
	/// # Errors
	///
	/// Returns an error when the artifact is invalid or its OGDL graph cannot be
	/// constructed.
	#[inline]
	pub fn encode(&self) -> CheckpointResult<Vec<u8>> { validate_artifact(self)?; let mut graph = Graph::new();
		let root = graph .append_root(ROOT)
			.map_err(|error| return CheckpointError::manifest(format!("encode KNN root: {error}")))?;
		field(&mut graph, root, "format-version", &self.format_version)?;
		field( &mut graph, root,
			"neighbors",
			&self.references.neighbors.get(), )?; field( &mut graph, root,
			"data-normalization",
			&self.data_normalization.map_or("none", |value| {
				return match value {
					DataNormalization::Identity => "identity",
					DataNormalization::ZScore => "z-score",
					DataNormalization::MinMax => "min-max",
					DataNormalization::L2Norm => "l2-norm",
				}; }), )?;
		let operations = child(&mut graph, root, "operations")?;
		for operation in &self.operations {
			field(&mut graph, operations, "operation", &operation.token())?;
		}
		let vectors = child(&mut graph, root, "vectors")?;
		for vector in &self.references.vectors { vector.encode_node(&mut graph, vectors)?; }
		let spans = child(&mut graph, root, "feature-spans")?;
		for span in &self.references.feature_spans {
			let node = child(&mut graph, spans, "span")?;
			field(&mut graph, node, "source-index", &span.source_vector())?;
			field(&mut graph, node, "start", &span.start())?;
			field(&mut graph, node, "width", &span.width())?;
			let lowering = child(&mut graph, node, "lowering")?;
			match span.lowering() { DenseFeatureLowering::NumericScalar => {
					child(&mut graph, lowering, "numeric-scalar")?;
				}
				DenseFeatureLowering::CategoricalOneHot { dictionary_width, reserved_index, } => {
					let tag = child(&mut graph, lowering, "categorical-one-hot")?;
					field(&mut graph, tag, "dictionary-width", &dictionary_width)?;
					field(&mut graph, tag, "reserved-index", &reserved_index)?;
				} } }
		let mask = child(&mut graph, root, "normalization-mask")?;
		match self.references.normalization_mask.as_deref() { Some(bits) => {
				let tag = child(&mut graph, mask, "f32-bits")?;
				scalar(&mut graph, tag, &encode_u32_hex(bits))?; }
			None => {
				child(&mut graph, mask, "none")?;
			} }
		let shape = child(&mut graph, root, "reference-shape")?;
		field(&mut graph, shape, "rows", &self.references.reference_rows)?;
		field( &mut graph, shape,
			"feature-width",
			&self.references.feature_width, )?;
		field(&mut graph, root, "reference-source-rows", &{
			const DIGITS: &[u8; 16] = b"0123456789abcdef";
			let values = &self.references.reference_source_rows;
			let mut encoded = String::with_capacity(2 + values.len().saturating_mul(16));
			encoded.push_str("0x");
			for value in values { let encoded_value = u64::try_from(*value).map_err(|error| {
					return CheckpointError::manifest(format!("KNN source row does not fit u64: {error}"));
				})?; for digit_index in (0..16).rev() { let shift = digit_index * 4;
					let digit = usize::try_from((encoded_value >> shift) & 0x0f).map_err(|error| {
						return CheckpointError::manifest(format!(
							"KNN source-row hexadecimal digit is invalid: {error}"
						)); })?; encoded.push(char::from(DIGITS[digit])); } }
			encoded })?; field( &mut graph, root,
			"reference-feature-f32-bits",
			&encode_u32_hex(&self.references.reference_feature_bits), )?;
		let outputs = child(&mut graph, root, "outputs")?;
		for output in &self.references.outputs {
			let node = child(&mut graph, outputs, "output")?;
			field( &mut graph, node,
				"source-index",
				&output.schema.source_index(), )?; let mut known_mask = String::with_capacity(2 + output.known.len());
			known_mask.push_str("0b");
			for value in &output.known {
				known_mask.push(if *value == 1 { '1' } else { '0' });
			}
			field(&mut graph, node, "known-mask", &known_mask)?;
			let values = child(&mut graph, node, "values")?;
			if output.values.is_numeric() {
					let tag = child(&mut graph, values, "numeric-f32-bits")?;
					scalar( &mut graph, tag, &encode_u32_hex(output.values.numeric_f32_bits_storage()), )?; } else {
					let tag = child(&mut graph, values, "discrete-int32")?;
					let (codes, labels) = output.values.discrete_i32_storage(); let encoded_codes = encode_u32_hex( &codes.iter()
							.map(|value| return value.cast_unsigned()) .collect::<Vec<_>>(), );
					field(&mut graph, tag, "codes", &encoded_codes)?;
					let dictionary = child(&mut graph, tag, "labels")?;
					for label in labels { if let Some(value) = label.as_i32() {
							field(&mut graph, dictionary, "int32", &value)?;
						} else {
							field(&mut graph, dictionary, "bytes", &encode_bytes(label.as_bytes().unwrap()))?;
						} } } }
		return Ok(graph.to_canonical_string().into_bytes()); }

	/// Decode and validate one bounded semantic KNN model image.
	///
	/// # Errors
	///
	/// Returns an error when the source is malformed, inconsistent, or exceeds
	/// the supplied decode limits.
	#[inline]
	pub fn decode(source: &[u8], limits: KnnModelDecodeLimits) -> CheckpointResult<Self> {
		return Decoder::new(source, limits)?.decode(); }

	/// Atomically persist this semantic model as its canonical textual OGDL
	/// image. The KNN reference state contains no native kernel bytes.
	///
	/// # Errors
	///
	/// Returns an error when the path is not an `.ogdl` target, encoding fails,
	/// or the artifact cannot be written atomically.
	#[inline]
	pub fn save(&self, path: impl AsRef<Path>) -> CheckpointResult<()> { let target = path.as_ref(); if target .extension()
			.and_then(|extension| return extension.to_str())
			!= Some("ogdl")
		{ return Err(CheckpointError::invalid_target( target,
				"KNN semantic model path must end in .ogdl",
			)); }
		let encoded = self.encode()?; let encoded_bytes = u64::try_from(encoded.len()).map_err(|error| {
			return CheckpointError::invalid_target(target, format!("KNN model size exceeds u64: {error}"));
		})?; return atomic_save(target, encoded_bytes, |file| { return file.write_all(&encoded); }); } }

/// Decode one bounded canonical semantic KNN model.
///
/// # Errors
///
/// Returns an error when the source is malformed, inconsistent, or exceeds the
/// supplied decode limits.
#[inline]
pub fn decode_knn_model(source: &[u8], limits: KnnModelDecodeLimits) -> CheckpointResult<KnnModelArtifact> {
	return Decoder::new(source, limits)?.decode(); }

/// Encode one semantic artifact component beneath an OGDL parent node.
trait EncodeKnnNode {
	/// Append the component and all of its canonical descendants.
	fn encode_node(&self, graph: &mut Graph, parent: NodeId) -> CheckpointResult<()>; }

impl EncodeKnnNode for CheckpointArtifactVector {
	fn encode_node(&self, graph: &mut Graph, parent: NodeId) -> CheckpointResult<()> {
		let node = child(graph, parent, "vector")?;
		field(graph, node, "source-index", &self.source_index())?;
		field(graph, node, "name-bytes", &encode_bytes(self.name()))?;
		field( graph, node,
			"role",
			&match self.role() {
				VectorRole::Feature => "feature",
				VectorRole::Target => "target",
			}, )?; field( graph, node,
			"semantic-type",
			&match self.semantic_type() {
				SemanticType::Numeric => "numeric",
				SemanticType::Temporal => "temporal",
				SemanticType::Categorical => "categorical",
				SemanticType::Ordinal => "ordinal",
				SemanticType::Text => "text",
				SemanticType::Image => "image",
				SemanticType::Binary => "binary",
			}, )?; field( graph, node,
			"encoding",
			&match self.encoding() {
				VectorEncoding::F32 => "f32",
				VectorEncoding::I32 => "int32",
				VectorEncoding::RelativeSecondsI32 => "relative-seconds-int32",
				VectorEncoding::DictionaryI32 => "dictionary-int32",
				VectorEncoding::OrdinalI32 => "ordinal-int32",
				VectorEncoding::Utf8 => "utf8",
				VectorEncoding::Bytes => "bytes",
			}, )?;
		let metadata = child(graph, node, "metadata")?;
		match self.metadata() { CheckpointArtifactMetadata::None => {
				child(graph, metadata, "none")?;
			}
			CheckpointArtifactMetadata::Temporal { unix_seconds, nanoseconds, } => {
				let tag = child(graph, metadata, "temporal")?;
				field(graph, tag, "unix-seconds", unix_seconds)?;
				field(graph, tag, "nanoseconds", nanoseconds)?;
			}
			CheckpointArtifactMetadata::Categorical { dictionary } => {
				let tag = child(graph, metadata, "categorical")?;
				for value in dictionary {
					field(graph, tag, "value-bytes", &encode_bytes(value))?;
				} }
			CheckpointArtifactMetadata::Ordinal { ordered_labels } => {
				let tag = child(graph, metadata, "ordinal")?;
				for value in ordered_labels {
					field(graph, tag, "value-bytes", &encode_bytes(value))?;
				} }
			CheckpointArtifactMetadata::Image { encoded_variants } => {
				let tag = child(graph, metadata, "image")?;
				for variant in encoded_variants {
					let item = child(graph, tag, "variant")?;
					field( graph, item,
						"format",
						&match variant.format() {
							EncodedImageFormat::Png => "png",
							EncodedImageFormat::Jpeg => "jpeg",
							EncodedImageFormat::Gif87a => "gif87a",
							EncodedImageFormat::Gif89a => "gif89a",
							EncodedImageFormat::Bmp => "bmp",
							EncodedImageFormat::WebP => "webp",
						}, )?;
					field(graph, item, "width", &variant.width())?;
					field(graph, item, "height", &variant.height())?;
					field( graph, item,
						"channels",
						&variant.channels().map_or_else(
							|| return "none".to_owned(),
							|value| return value.to_string(), ), )?; field( graph, item,
						"color-model",
						&variant.color_model().map_or("none", |value| {
							return match value {
								ImageColorModel::Grayscale => "grayscale",
								ImageColorModel::GrayscaleAlpha => "grayscale-alpha",
								ImageColorModel::Rgb => "rgb",
								ImageColorModel::Rgba => "rgba",
								ImageColorModel::Bgr => "bgr",
								ImageColorModel::IndexedRgb => "indexed-rgb",
								ImageColorModel::YCbCr => "y-cb-cr",
								ImageColorModel::Cmyk => "cmyk",
								ImageColorModel::Ycck => "ycck",
							}; }), )?; field( graph, item,
						"sample-bits",
						&variant.sample_bits().map_or_else(
							|| return "none".to_owned(),
							|value| return value.to_string(), ), )?; field( graph, item,
						"value-layout",
						&match variant.value_layout() {
							ImageValueLayout::EncodedFile => "encoded-file",
						}, )?; field( graph, item,
						"value-range",
						&match variant.value_range() {
							ImageValueRange::EncodedBytes => "encoded-bytes",
						}, )?; } } }
		return Ok(()); } }

/// Append one child node while mapping OGDL errors into checkpoint errors.
fn child(graph: &mut Graph, parent: NodeId, text: impl Into<String>) -> CheckpointResult<NodeId> { return graph
		.append_child(parent, text)
		.map_err(|error| return CheckpointError::manifest(format!("encode KNN OGDL node: {error}")));
}

/// Append one scalar child value.
fn scalar(graph: &mut Graph, parent: NodeId, value: &impl ToString) -> CheckpointResult<NodeId> {
	return child(graph, parent, value.to_string()); }

/// Append one named scalar field.
fn field(graph: &mut Graph, parent: NodeId, name: impl Into<String>, value: &impl ToString) -> CheckpointResult<()> {
	let node = child(graph, parent, name)?; scalar(graph, node, value)?; return Ok(()); }

/// Encode arbitrary bytes as canonical lowercase hexadecimal.
fn encode_bytes(bytes: &[u8]) -> String { let mut output = String::with_capacity(2 + bytes.len().saturating_mul(2));
	output.push_str("0x");
	append_hex_bytes(&mut output, bytes); return output; }

/// Encode `u32` values as canonical fixed-width lowercase hexadecimal.
fn encode_u32_hex(values: &[u32]) -> String {
	let mut output = String::with_capacity(2 + values.len().saturating_mul(8));
	output.push_str("0x");
	for value in values { append_hex_bytes(&mut output, &value.to_be_bytes()); }
	return output; }

/// Append bytes as lowercase hexadecimal without fallible formatting.
#[inline]
fn append_hex_bytes(output: &mut String, bytes: &[u8]) {
	const DIGITS: &[u8; 16] = b"0123456789abcdef";
	for byte in bytes { output.push(char::from(DIGITS[usize::from(*byte >> 4)]));
		output.push(char::from(DIGITS[usize::from(*byte & 0x0f)])); }
	return; }

/// Validate one complete semantic KNN artifact.
fn validate_artifact(artifact: &KnnModelArtifact) -> CheckpointResult<()> {
	if artifact.format_version != KNN_MODEL_FORMAT_VERSION { return Err(CheckpointError::manifest(format!(
			"KNN model format version {} is not supported",
			artifact.format_version ))); }
	let references = &artifact.references; if references.reference_rows == 0 || references.feature_width == 0 {
		return Err(CheckpointError::manifest("KNN reference rows and feature width must both be nonzero"));
	}
	let feature_elements = references .reference_rows .checked_mul(references.feature_width)
		.ok_or_else(|| return CheckpointError::manifest("KNN reference feature shape overflowed usize"))?;
	if references.reference_feature_bits.len() != feature_elements { return Err(CheckpointError::manifest(format!(
			"KNN reference feature image has {} elements, expected {feature_elements}",
			references.reference_feature_bits.len() ))); }
	if references .reference_feature_bits .iter() .any(|bits| return !f32::from_bits(*bits).is_finite())
	{ return Err(CheckpointError::manifest(
			"KNN reference features contain a non-finite f32",
		)); }
	if references.reference_source_rows.len() != references.reference_rows { return Err(CheckpointError::manifest(
			"KNN reference-source-row count differs from the reference row count",
		)); }
	if references.vectors.is_empty() {
		return Err(CheckpointError::manifest("KNN saved vector schema is empty"));
	}
	let mut source_indices = BTreeSet::new(); let mut names = BTreeSet::new(); let mut previous = None;
	for (index, vector) in references.vectors.iter().enumerate() { if !source_indices.insert(vector.source_index()) {
			return Err(CheckpointError::manifest(format!(
				"KNN vector source index {} appears more than once",
				vector.source_index() ))); }
		if previous.is_some_and(|prior| return prior >= vector.source_index()) { return Err(CheckpointError::manifest(
				"KNN vector schemas must retain strictly increasing source order",
			)); }
		previous = Some(vector.source_index()); if vector.name().is_empty() || !names.insert(vector.name()) {
			return Err(CheckpointError::manifest(
				"KNN vector names must be nonempty and unique",
			)); }
		validate_saved_vector( vector,
			&CheckpointPath::root().field("vectors").index(index),
		)?; }
	if references.feature_spans.is_empty() {
		return Err(CheckpointError::manifest("KNN feature span list is empty"));
	}
	let feature_sources = references .vectors .iter() .filter(|vector| return vector.role() == VectorRole::Feature)
		.map(CheckpointArtifactVector::source_index) .collect::<BTreeSet<_>>(); let mut seen_spans = BTreeSet::new();
	let mut cursor = 0usize; for span in &references.feature_spans { if span.start() != cursor || span.width() == 0 {
			return Err(CheckpointError::manifest("KNN feature spans must be nonempty and contiguous from zero"));
		}
		if !feature_sources.contains(&span.source_vector()) || !seen_spans.insert(span.source_vector()) {
			return Err(CheckpointError::manifest("KNN feature span source is absent, duplicated, or not a feature"));
		}
		match span.lowering() { DenseFeatureLowering::NumericScalar if span.width() == 1 => {}
			DenseFeatureLowering::CategoricalOneHot { dictionary_width, reserved_index, } => {
				if reserved_index != dictionary_width {
					return Err(CheckpointError::manifest("KNN categorical span reserved index differs from dictionary width"));
				}
				if span.width() != dictionary_width.saturating_add(1) {
					return Err(CheckpointError::manifest("KNN categorical span width differs from encoded dictionary width"));
				} }
			DenseFeatureLowering::NumericScalar => {
				return Err(CheckpointError::manifest("KNN feature span lowering is internally inconsistent"));
			} }
		cursor = cursor .checked_add(span.width())
			.ok_or_else(|| return CheckpointError::manifest("KNN feature span width overflowed usize"))?;
	}
	if cursor != references.feature_width || seen_spans != feature_sources {
		return Err(CheckpointError::manifest("KNN feature spans do not exactly cover every saved feature column"));
	}
	if references.normalization_mask.as_ref().is_some_and(|mask| { return mask.len() != references.feature_width || mask
				.iter() .any(|bits| return !matches!(*bits, 0x0000_0000 | 0x3f80_0000)); }) { return Err(CheckpointError::manifest(
			"KNN normalization mask must contain one exact positive-zero/one f32 bit per feature column",
		)); }
	if references.outputs.is_empty() {
		return Err(CheckpointError::manifest("KNN model has no declared outputs"));
	}
	let vectors = references .vectors .iter() .map(|vector| return (vector.source_index(), vector))
		.collect::<BTreeMap<_, _>>(); let mut seen = BTreeSet::new(); for output in &references.outputs {
		let source = output.schema.source_index(); if !seen.insert(source)
			|| vectors.get(&source).copied() != Some(&output.schema)
			|| output.schema.role() != VectorRole::Target
		{ return Err(CheckpointError::manifest(
				"KNN output schemas must be distinct exact target schemas from the saved vector list",
			)); }
		if output.known.len() != references.reference_rows || output .known .iter()
				.any(|value| return !matches!(*value, 0 | 1))
		{ return Err(CheckpointError::manifest(
				"KNN output known mask is not one binary value per reference row",
			)); }
		let count = u64::try_from( output.known .iter() .filter(|value| return **value == 1) .count(), ) .map_err(|error| {
			return CheckpointError::manifest(format!("KNN known count does not fit u64: {error}"));
		})?; if count == 0 || count != output.known_references { return Err(CheckpointError::manifest(
				"KNN output known-reference count is zero or differs from its mask",
			)); }
		if output.values.is_numeric() { let bits = output.values.numeric_f32_bits_storage();
				if bits.len() != references.reference_rows || bits .iter()
						.any(|encoded_bits| return !f32::from_bits(*encoded_bits).is_finite())
				{ return Err(CheckpointError::manifest(
						"KNN numeric output must contain one finite f32 per reference row",
					)); } } else { let (codes, labels) = output.values.discrete_i32_storage();
				if codes.len() != references.reference_rows || labels.is_empty() || labels.len() > i32::MAX as usize
				{ return Err(CheckpointError::manifest(
						"KNN discrete output has an invalid row or label count",
					)); }
				let mut unique = BTreeSet::new(); if labels.iter().any(|label| return !unique.insert(label)) {
					return Err(CheckpointError::manifest(
						"KNN discrete output labels are not unique",
					)); }
				if codes.iter().any(|code| { return usize::try_from(*code).map_or(true, |index| return index >= labels.len()); }) {
					return Err(CheckpointError::manifest(
						"KNN discrete output contains an invalid class code",
					)); } } }
	return Ok(()); }

/// Construct an incompatible-resume checkpoint error.
fn incompatible_resume(detail: impl Into<String>) -> CheckpointError { return CheckpointError::IncompatibleResume {
		detail: detail.into(), }; }

#[derive(Debug)]
/// Stateful bounded decoder for one semantic KNN artifact.
struct Decoder {
	/// Parsed OGDL graph.
	graph: Graph,
	/// Caller-provided allocation and structural bounds.
	limits: KnnModelDecodeLimits,
	/// Total decoded payload bytes charged so far.
	payload_bytes: usize,
	/// Total decoded metadata entries charged so far.
	metadata_entries: usize,
	/// Total decoded discrete labels charged so far.
	labels: usize, }

impl Decoder {
	/// Parse source bytes and establish bounded decoder accounting.
	fn new(source: &[u8], limits: KnnModelDecodeLimits) -> CheckpointResult<Self> { let root = CheckpointPath::root();
		if source.len() > limits.source_bytes { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded, root,
				format!(
					"KNN model source has {} bytes, limit is {}",
					source.len(), limits.source_bytes ), )); }
		let node_bound = if source.is_empty() { 0 } else { 1usize.checked_add( source.iter()
					.filter(|byte| return matches!(**byte, b'\n' | b'\t'))
					.count(), ) .ok_or_else(|| { return decode_error( CheckpointDecodeErrorKind::LimitExceeded, CheckpointPath::root(),
					"KNN model node pre-count overflowed usize",
				); })? }; if node_bound > limits.nodes { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded,
				CheckpointPath::root(), format!(
					"KNN model has at least {node_bound} nodes, limit is {}",
					limits.nodes ), )); }
		let text = str::from_utf8(source).map_err(|error| { return decode_error( CheckpointDecodeErrorKind::InvalidUtf8,
				CheckpointPath::root(),
				format!("KNN model is not UTF-8: {error}"),
			); })?; let graph = Graph::parse(text).map_err(|error| { return decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax, CheckpointPath::root(),
				format!("invalid KNN OGDL: {error}"),
			); })?; if graph.len() > limits.nodes { return Err(decode_error( CheckpointDecodeErrorKind::LimitExceeded,
				CheckpointPath::root(), format!(
					"KNN model has {} nodes, limit is {}",
					graph.len(), limits.nodes ), )); }
		return Ok(Self { graph, limits, payload_bytes: 0, metadata_entries: 0, labels: 0, }); }

	/// Decode and validate the complete semantic KNN artifact.
	fn decode(mut self) -> CheckpointResult<KnnModelArtifact> { let path = CheckpointPath::root();
		let roots = self.graph.roots(); if roots.len() != 1 { return Err(decode_error(
				CheckpointDecodeErrorKind::InvalidSyntax, path,
				format!("KNN model requires exactly one root, found {}", roots.len()),
			)); }
		let root = roots.first().copied().ok_or_else(|| { return decode_error( CheckpointDecodeErrorKind::InvalidSyntax,
				path.clone(),
				"KNN model root is absent",
			); })?; if self.node(root, &path)?.text() != ROOT { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
				path,
				format!("KNN model root must be {ROOT:?}"),
			)); }
		let fields = self.fields( root, &path, &[
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
			], )?;
		let format_version_path = path.field("format-version");
		let format_version = Self::parse_u32(self.scalar(fields["format-version"], &format_version_path)?, &format_version_path)?;
		if format_version != KNN_MODEL_FORMAT_VERSION { return Err(Self::invalid_value( &format_version_path,
				format!("unsupported KNN model version {format_version}"),
			)); }
		let neighbors_path = path.field("neighbors");
		let neighbor_count = parse_canonical(self.scalar(fields["neighbors"], &neighbors_path)?, &neighbors_path)?;
		let neighbors = NonZeroU64::new(neighbor_count).ok_or_else(|| { return Self::invalid_value( &neighbors_path,
				"neighbor count must be nonzero",
			); })?;
		let data_normalization_path = path.field("data-normalization");
		let data_normalization = match self.scalar(fields["data-normalization"], &data_normalization_path)? {
			"none" => None,
			"z-score" => Some(DataNormalization::ZScore),
			"min-max" => Some(DataNormalization::MinMax),
			"l2-norm" => Some(DataNormalization::L2Norm),
			_ => { return Err(Self::invalid_value( &data_normalization_path,
					"unknown data normalization",
				)); } }; let operations = self.parse_operations(
			fields["operations"],
			&path.field("operations"),
		)?;
		let vectors = self.parse_vectors(fields["vectors"], &path.field("vectors"))?;
		let feature_spans = self.parse_spans(
			fields["feature-spans"],
			&path.field("feature-spans"),
		)?; let normalization_mask = self.parse_mask(
			fields["normalization-mask"],
			&path.field("normalization-mask"),
		)?;
		let shape_path = path.field("reference-shape");
		let shape = self.fields(
			fields["reference-shape"],
			&shape_path,
			&["rows", "feature-width"],
		)?;
		let rows_path = shape_path.field("rows");
		let reference_rows = Self::parse_usize(self.scalar(shape["rows"], &rows_path)?, &rows_path)?;
		let width_path = shape_path.field("feature-width");
		let feature_width = Self::parse_usize(self.scalar(shape["feature-width"], &width_path)?, &width_path)?;
		if reference_rows == 0 || reference_rows > self.limits.reference_rows { return Err(Self::limit( &rows_path, format!(
					"reference row count is {reference_rows}, limit is {}",
					self.limits.reference_rows ), )); }
		if feature_width == 0 || feature_width > self.limits.feature_width { return Err(Self::limit( &width_path, format!(
					"feature width is {feature_width}, limit is {}",
					self.limits.feature_width ), )); }
		let reference_source_rows_value = self .scalar(
				fields["reference-source-rows"],
				&CheckpointPath::root().field("reference-source-rows"),
			)? .to_owned(); let reference_source_rows = self.parse_usize_hex( &reference_source_rows_value, reference_rows,
			&CheckpointPath::root().field("reference-source-rows"),
		)?; let feature_elements = reference_rows.checked_mul(feature_width).ok_or_else(|| { return Self::limit( &shape_path,
				"reference feature element count overflowed usize",
			); })?; let reference_feature_bits_value = self .scalar(
				fields["reference-feature-f32-bits"],
				&CheckpointPath::root().field("reference-feature-f32-bits"),
			)? .to_owned(); let reference_feature_bits = self.parse_u32_hex( &reference_feature_bits_value, feature_elements,
			&CheckpointPath::root().field("reference-feature-f32-bits"),
		)?; let outputs = self.parse_outputs(
			fields["outputs"],
			&CheckpointPath::root().field("outputs"),
			&vectors, reference_rows, )?; let artifact = KnnModelArtifact { format_version, references: KnnReferenceSet {
				neighbors, vectors, feature_spans, normalization_mask, reference_source_rows, reference_rows, feature_width,
				reference_feature_bits, outputs, }, data_normalization, operations, };
		validate_artifact(&artifact).map_err(|validation_error| match validation_error {
			CheckpointError::Decode(decode_failure) => return CheckpointError::Decode(decode_failure),
			other @ (CheckpointError::InvalidManifest { .. }
			| CheckpointError::IncompatibleResume { .. }
			| CheckpointError::DuplicateOutput { .. }
			| CheckpointError::MissingOutput { .. }
			| CheckpointError::UnexpectedOutput { .. }
			| CheckpointError::OutputDTypeMismatch { .. }
			| CheckpointError::OutputSizeMismatch { .. }
			| CheckpointError::NativeKernelUnavailable { .. }
			| CheckpointError::NativeKernelAmbiguous { .. }
			| CheckpointError::InvalidTarget { .. }
			| CheckpointError::InsufficientCapacity { .. }
			| CheckpointError::Io { .. }) => { return decode_error( CheckpointDecodeErrorKind::InconsistentValue,
					CheckpointPath::root(), other.to_string(), ); } })?; return Ok(artifact); }

	/// Decode the ordered dense-operation list.
	fn parse_operations(&self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<Vec<Function>> {
		let children = self.node(node, path)?.children(); let mut operations = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() { let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "operation" {
				return Err(Self::unknown(&item_path, "expected operation entry"));
			}
			let value = self.scalar(child, &item_path)?; operations.push(Function::from_token(value).ok_or_else(|| {
				return Self::invalid_value(&item_path, format!("unknown dense operation {value:?}"));
			})?); }
		return Ok(operations); }

	/// Decode saved vector schemas in declaration order.
	fn parse_vectors( &mut self, node: NodeId, path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointArtifactVector>> { let children = self.node(node, path)?.children().to_vec();
		if children.len() > self.limits.vectors { return Err(Self::limit( path, format!(
					"vector count is {}, limit is {}",
					children.len(), self.limits.vectors ), )); }
		let mut vectors = Vec::with_capacity(children.len()); for (index, child) in children.into_iter().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "vector" {
				return Err(Self::unknown(&item_path, "expected vector entry"));
			}
			let fields = self.fields( child, &item_path, &[
					"source-index",
					"name-bytes",
					"role",
					"semantic-type",
					"encoding",
					"metadata",
				], )?; let source_index = Self::parse_usize(
				self.scalar(fields["source-index"], &item_path.field("source-index"))?,
				&item_path.field("source-index"),
			)?; let name_value = self
				.scalar(fields["name-bytes"], &item_path.field("name-bytes"))?
				.to_owned();
			let name = self.parse_bytes(&name_value, &item_path.field("name-bytes"))?;
			let role_path = item_path.field("role");
			let role = match self.scalar(fields["role"], &role_path)? {
				"feature" => VectorRole::Feature,
				"target" => VectorRole::Target,
				_ => return Err(Self::invalid_value(&role_path, "unknown vector role")),
			};
			let semantic_path = item_path.field("semantic-type");
			let semantic_type = match self.scalar(fields["semantic-type"], &semantic_path)? {
				"numeric" => SemanticType::Numeric,
				"temporal" => SemanticType::Temporal,
				"categorical" => SemanticType::Categorical,
				"ordinal" => SemanticType::Ordinal,
				"text" => SemanticType::Text,
				"image" => SemanticType::Image,
				"binary" => SemanticType::Binary,
				_ => return Err(Self::invalid_value(&semantic_path, "unknown semantic type")),
			};
			let encoding_path = item_path.field("encoding");
			let encoding = match self.scalar(fields["encoding"], &encoding_path)? {
				"f32" => VectorEncoding::F32,
				"int32" => VectorEncoding::I32,
				"relative-seconds-int32" => VectorEncoding::RelativeSecondsI32,
				"dictionary-int32" => VectorEncoding::DictionaryI32,
				"ordinal-int32" => VectorEncoding::OrdinalI32,
				"utf8" => VectorEncoding::Utf8,
				"bytes" => VectorEncoding::Bytes,
				_ => { return Err(Self::invalid_value( &encoding_path,
						"unknown vector encoding",
					)); } };
			let metadata = self.parse_metadata(fields["metadata"], &item_path.field("metadata"))?;
			vectors.push(CheckpointArtifactVector::new( source_index, name, role, semantic_type, encoding, metadata, )); }
		return Ok(vectors); }

	/// Decode the metadata attached to one saved vector.
	fn parse_metadata( &mut self, node: NodeId, path: &CheckpointPath, ) -> CheckpointResult<CheckpointArtifactMetadata> {
		let tag = self.tag(node, path)?; let text = self.node(tag, path)?.text().to_owned();
		let children = self.node(tag, path)?.children().to_vec(); return match text.as_str() {
			"none" => {
				Self::require_empty(&children, path)?; Ok(CheckpointArtifactMetadata::None) }
			"temporal" => {
				let fields = self.fields_from(&children, path, &["unix-seconds", "nanoseconds"])?;
				let unix_seconds = parse_canonical(
					self.scalar(fields["unix-seconds"], &path.field("unix-seconds"))?,
					&path.field("unix-seconds"),
				)?; let nanoseconds = Self::parse_u32(
					self.scalar(fields["nanoseconds"], &path.field("nanoseconds"))?,
					&path.field("nanoseconds"),
				)?; Ok(CheckpointArtifactMetadata::Temporal { unix_seconds, nanoseconds, }) }
			"categorical" => Ok(CheckpointArtifactMetadata::Categorical {
				dictionary: self.parse_byte_entries(&children, path)?, }),
			"ordinal" => Ok(CheckpointArtifactMetadata::Ordinal {
				ordered_labels: self.parse_byte_entries(&children, path)?, }),
			"image" => Ok(CheckpointArtifactMetadata::Image {
				encoded_variants: self.parse_image_entries(&children, path)?, }), _ => Err(Self::invalid_value( path,
				format!("unknown vector metadata {text:?}"),
			)), }; }

	/// Decode a bounded sequence of byte-valued metadata entries.
	fn parse_byte_entries(&mut self, children: &[NodeId], path: &CheckpointPath) -> CheckpointResult<Vec<Vec<u8>>> {
		self.reserve_metadata(children.len(), path)?; let mut values = Vec::with_capacity(children.len());
		for (index, child) in children.iter().copied().enumerate() { let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "value-bytes" {
				return Err(Self::unknown( &item_path,
					"expected value-bytes metadata entry",
				)); }
			let value = self.scalar(child, &item_path)?.to_owned(); values.push(self.parse_bytes(&value, &item_path)?); }
		return Ok(values); }

	/// Decode a bounded sequence of encoded-image metadata entries.
	fn parse_image_entries( &mut self, children: &[NodeId], path: &CheckpointPath,
	) -> CheckpointResult<Vec<CheckpointImageMetadata>> { self.reserve_metadata(children.len(), path)?;
		let mut values = Vec::with_capacity(children.len()); for (index, child) in children.iter().copied().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "variant" {
				return Err(Self::unknown(&item_path, "expected image variant"));
			}
			let fields = self.fields( child, &item_path, &[
					"format",
					"width",
					"height",
					"channels",
					"color-model",
					"sample-bits",
					"value-layout",
					"value-range",
				], )?;
			let format_path = item_path.field("format");
			let format = match self.scalar(fields["format"], &format_path)? {
				"png" => EncodedImageFormat::Png,
				"jpeg" => EncodedImageFormat::Jpeg,
				"gif87a" => EncodedImageFormat::Gif87a,
				"gif89a" => EncodedImageFormat::Gif89a,
				"bmp" => EncodedImageFormat::Bmp,
				"webp" => EncodedImageFormat::WebP,
				_ => return Err(Self::invalid_value(&format_path, "unknown image format")),
			}; let width = Self::parse_u32(
				self.scalar(fields["width"], &item_path.field("width"))?,
				&item_path.field("width"),
			)?; let height = Self::parse_u32(
				self.scalar(fields["height"], &item_path.field("height"))?,
				&item_path.field("height"),
			)?; let channels = Self::parse_optional_u8(
				self.scalar(fields["channels"], &item_path.field("channels"))?,
				&item_path.field("channels"),
			)?;
			let color_model_path = item_path.field("color-model");
			let color_model = match self.scalar(fields["color-model"], &color_model_path)? {
				"none" => None,
				"grayscale" => Some(ImageColorModel::Grayscale),
				"grayscale-alpha" => Some(ImageColorModel::GrayscaleAlpha),
				"rgb" => Some(ImageColorModel::Rgb),
				"rgba" => Some(ImageColorModel::Rgba),
				"bgr" => Some(ImageColorModel::Bgr),
				"indexed-rgb" => Some(ImageColorModel::IndexedRgb),
				"y-cb-cr" => Some(ImageColorModel::YCbCr),
				"cmyk" => Some(ImageColorModel::Cmyk),
				"ycck" => Some(ImageColorModel::Ycck),
				_ => { return Err(Self::invalid_value( &color_model_path,
						"unknown image color model",
					)); } }; let sample_bits = Self::parse_optional_u8(
				self.scalar(fields["sample-bits"], &item_path.field("sample-bits"))?,
				&item_path.field("sample-bits"),
			)?; Self::expect(
				self.scalar(fields["value-layout"], &item_path.field("value-layout"))?,
				"encoded-file",
				&item_path.field("value-layout"),
			)?; Self::expect(
				self.scalar(fields["value-range"], &item_path.field("value-range"))?,
				"encoded-bytes",
				&item_path.field("value-range"),
			)?; values.push(CheckpointImageMetadata::new( format, width, height, channels, color_model, sample_bits, )); }
		return Ok(values); }

	/// Decode the complete dense feature-span layout.
	fn parse_spans(&self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<Vec<CompiledFeatureSpan>> {
		let children = self.node(node, path)?.children(); if children.len() > self.limits.feature_spans {
			return Err(Self::limit( path, format!(
					"feature-span count is {}, limit is {}",
					children.len(), self.limits.feature_spans ), )); }
		let mut spans = Vec::with_capacity(children.len()); for (index, child) in children.iter().copied().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "span" {
				return Err(Self::unknown(&item_path, "expected feature span"));
			}
			let fields = self.fields( child, &item_path,
				&["source-index", "start", "width", "lowering"],
			)?; let source_index = Self::parse_usize(
				self.scalar(fields["source-index"], &item_path.field("source-index"))?,
				&item_path.field("source-index"),
			)?; let start = Self::parse_usize(
				self.scalar(fields["start"], &item_path.field("start"))?,
				&item_path.field("start"),
			)?; let width = Self::parse_usize(
				self.scalar(fields["width"], &item_path.field("width"))?,
				&item_path.field("width"),
			)?;
			let lowering_tag = self.tag(fields["lowering"], &item_path.field("lowering"))?;
			let lowering_name = self
				.node(lowering_tag, &item_path.field("lowering"))?
				.text(); let lowering = match lowering_name {
				"numeric-scalar" => {
					Self::require_empty( self.node(lowering_tag, &item_path)?.children(),
						&item_path.field("lowering"),
					)?; DenseFeatureLowering::NumericScalar }
				"categorical-one-hot" => {
					let tagged_fields = self.fields( lowering_tag,
						&item_path.field("lowering"),
						&["dictionary-width", "reserved-index"],
					)?; DenseFeatureLowering::CategoricalOneHot { dictionary_width: Self::parse_usize( self.scalar(
								tagged_fields["dictionary-width"],
								&item_path.field("lowering").field("dictionary-width"),
							)?,
							&item_path.field("lowering").field("dictionary-width"),
						)?, reserved_index: Self::parse_usize( self.scalar(
								tagged_fields["reserved-index"],
								&item_path.field("lowering").field("reserved-index"),
							)?,
							&item_path.field("lowering").field("reserved-index"),
						)?, } }
				_ => { return Err(Self::invalid_value(
						&item_path.field("lowering"),
						"unknown feature lowering",
					)); } }; spans.push(CompiledFeatureSpan::new( source_index, start, width, lowering, )); }
		return Ok(spans); }

	/// Decode the optional normalization-mask payload.
	fn parse_mask(&mut self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<Option<Vec<u32>>> {
		let tag = self.tag(node, path)?; return match self.node(tag, path)?.text() {
			"none" => {
				Self::require_empty(self.node(tag, path)?.children(), path)?; Ok(None) }
			"f32-bits" => {
				let scalar = self.scalar(tag, path)?.to_owned(); let count = scalar
					.strip_prefix("0x")
					.map_or(0, |digits| return digits.len() / 8); self.parse_u32_hex(&scalar, count, path).map(Some) }
			_ => Err(Self::invalid_value(path, "unknown normalization-mask tag")),
		}; }

	/// Decode saved target outputs and their reference-row payloads.
	fn parse_outputs( &mut self, node: NodeId, path: &CheckpointPath, vectors: &[CheckpointArtifactVector],
		reference_rows: usize, ) -> CheckpointResult<Vec<KnnReferenceOutput>> {
		let children = self.node(node, path)?.children().to_vec();
		if children.is_empty() || children.len() > self.limits.outputs { return Err(Self::limit( path, format!(
					"output count is {}, limit is {}",
					children.len(), self.limits.outputs ), )); }
		let schemas = vectors .iter() .filter(|vector| return vector.role() == VectorRole::Target)
			.map(|vector| return (vector.source_index(), vector)) .collect::<BTreeMap<_, _>>();
		let mut outputs = Vec::with_capacity(children.len()); for (index, child) in children.into_iter().enumerate() {
			let item_path = path.index(index);
			if self.node(child, &item_path)?.text() != "output" {
				return Err(Self::unknown(&item_path, "expected output entry"));
			}
			let fields = self.fields(child, &item_path, &["source-index", "known-mask", "values"])?;
			let source_index = Self::parse_usize(
				self.scalar(fields["source-index"], &item_path.field("source-index"))?,
				&item_path.field("source-index"),
			)?; let schema = schemas.get(&source_index).copied().ok_or_else(|| { return Self::invalid_value(
					&item_path.field("source-index"),
					"output source is not a saved target vector",
				); })?; let known_value = self
				.scalar(fields["known-mask"], &item_path.field("known-mask"))?
				.to_owned();
			let known = self.parse_known(&known_value, reference_rows, &item_path.field("known-mask"))?;
			let known_references = u64::try_from(known.iter().filter(|value| return **value == 1).count()) .map_err(|error| {
					return Self::invalid_value(
						&item_path.field("known-mask"),
						format!("known count does not fit u64: {error}"),
					); })?;
			let tag = self.tag(fields["values"], &item_path.field("values"))?;
			let values = match self.node(tag, &item_path.field("values"))?.text() {
				"numeric-f32-bits" => {
					let value = self.scalar(tag, &item_path.field("values"))?.to_owned();
					KnnReferenceValues::from_numeric_f32_bits(self.parse_u32_hex( &value, reference_rows,
						&item_path.field("values"),
					)?) }
				"discrete-int32" => {
					let value_fields = self.fields(tag, &item_path.field("values"), &["codes", "labels"])?;
					let codes_value = self .scalar(
							value_fields["codes"],
							&item_path.field("values").field("codes"),
						)? .to_owned(); let codes = self.parse_i32_hex( &codes_value, reference_rows,
						&item_path.field("values").field("codes"),
					)?; let labels = self.parse_labels(
						value_fields["labels"],
						&item_path.field("values").field("labels"),
					)?; KnnReferenceValues::from_discrete_i32(codes, labels) }
				_ => { return Err(Self::invalid_value(
						&item_path.field("values"),
						"unknown KNN output value kind",
					)); } }; outputs.push(KnnReferenceOutput { schema: schema.clone(), known, known_references, values, }); }
		return Ok(outputs); }

	/// Decode and account for one discrete-label dictionary.
	fn parse_labels(&mut self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<Vec<KnnLabelValue>> {
		let children = self.node(node, path)?.children().to_vec(); self.labels = self .labels .checked_add(children.len())
			.ok_or_else(|| return Self::limit(path, "label count overflowed"))?;
		if self.labels > self.limits.labels { return Err(Self::limit( path, format!(
					"label count is {}, limit is {}",
					self.labels, self.limits.labels ), )); }
		let mut labels = Vec::with_capacity(children.len()); for (index, child) in children.into_iter().enumerate() {
			let item_path = path.index(index); let kind = self.node(child, &item_path)?.text().to_owned();
			let value = self.scalar(child, &item_path)?.to_owned(); labels.push(match kind.as_str() {
				"int32" => KnnLabelValue::i32(parse_canonical(&value, &item_path)?),
				"bytes" => KnnLabelValue::bytes(self.parse_bytes(&value, &item_path)?),
				_ => return Err(Self::unknown(&item_path, "expected int32 or bytes label")),
			}); }
		return Ok(labels); }

	/// Decode one node's required named fields.
	fn fields( &self, node: NodeId, path: &CheckpointPath, required: &[&str],
	) -> CheckpointResult<BTreeMap<String, NodeId>> {
		return self.fields_from(self.node(node, path)?.children(), path, required); }

	/// Decode required named fields from a child-node sequence.
	fn fields_from( &self, children: &[NodeId], path: &CheckpointPath, required: &[&str],
	) -> CheckpointResult<BTreeMap<String, NodeId>> { let allowed = required.iter().copied().collect::<BTreeSet<_>>();
		let mut fields = BTreeMap::new(); for child in children { let name = self.node(*child, path)?.text();
			if !allowed.contains(name) { return Err(Self::unknown( &path.field(name),
					format!("unknown field {name:?}"),
				)); }
			if fields.insert(name.to_owned(), *child).is_some() { return Err(decode_error(
					CheckpointDecodeErrorKind::DuplicateField, path.field(name),
					format!("field {name:?} appears more than once"),
				)); } }
		for name in required { if !fields.contains_key(*name) { return Err(decode_error(
					CheckpointDecodeErrorKind::MissingField, path.field(*name),
					format!("required field {name:?} is absent"),
				)); } }
		return Ok(fields); }

	/// Resolve one node identity or report invalid graph structure.
	fn node(&self, id: NodeId, path: &CheckpointPath) -> CheckpointResult<&recipe_ogdl::Node> {
		return self.graph.node(id).ok_or_else(|| { return decode_error( CheckpointDecodeErrorKind::InvalidSyntax,
				path.clone(),
				"KNN OGDL contains an unknown node identity",
			); }); }

	/// Decode one scalar child value.
	fn scalar<'a>(&'a self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<&'a str> {
		let children = self.node(node, path)?.children(); if children.len() != 1 { return Err(Self::invalid_value( path,
				format!("scalar field has {} values", children.len()),
			)); }
		let value_node = children .first() .copied()
			.ok_or_else(|| return Self::invalid_value(path, "scalar value is absent"))?;
		let scalar_value = self.node(value_node, path)?; if !scalar_value.children().is_empty() {
			return Err(Self::invalid_value(path, "scalar value has descendants"));
		}
		return Ok(scalar_value.text()); }

	/// Decode one tagged child node.
	fn tag(&self, node: NodeId, path: &CheckpointPath) -> CheckpointResult<NodeId> {
		let children = self.node(node, path)?.children(); if children.len() != 1 { return Err(Self::invalid_value( path,
				format!("tagged field has {} tags", children.len()),
			)); }
		let tag = children .first() .copied()
			.ok_or_else(|| return Self::invalid_value(path, "tagged value is absent"))?;
		return Ok(tag); }

	/// Require that a tagged value has no descendants.
	fn require_empty(children: &[NodeId], path: &CheckpointPath) -> CheckpointResult<()> { return if children.is_empty() {
			Ok(()) } else {
			Err(Self::unknown(path, "tag has unexpected descendants"))
		}; }

	/// Charge decoded payload bytes against the configured bound.
	fn reserve_payload(&mut self, bytes: usize, path: &CheckpointPath) -> CheckpointResult<()> { self.payload_bytes = self
			.payload_bytes .checked_add(bytes)
			.ok_or_else(|| return Self::limit(path, "payload byte count overflowed"))?;
		if self.payload_bytes > self.limits.total_payload_bytes { return Err(Self::limit( path, format!(
					"decoded payload is {} bytes, limit is {}",
					self.payload_bytes, self.limits.total_payload_bytes ), )); }
		return Ok(()); }

	/// Charge decoded metadata entries against the configured bound.
	fn reserve_metadata(&mut self, entries: usize, path: &CheckpointPath) -> CheckpointResult<()> {
		self.metadata_entries = self .metadata_entries .checked_add(entries)
			.ok_or_else(|| return Self::limit(path, "metadata entry count overflowed"))?;
		if self.metadata_entries > self.limits.metadata_entries { return Err(Self::limit( path, format!(
					"metadata entry count is {}, limit is {}",
					self.metadata_entries, self.limits.metadata_entries ), )); }
		return Ok(()); }

	/// Decode a canonical hexadecimal byte string.
	fn parse_bytes(&mut self, value: &str, path: &CheckpointPath) -> CheckpointResult<Vec<u8>> {
		let digits = canonical_hex(value, path)?; if digits.len() % 2 != 0 { return Err(Self::invalid_value( path,
				"hex byte string has an odd digit count",
			)); }
		let bytes = digits.len() / 2; self.reserve_payload(bytes, path)?; return (0..bytes) .map(|index| {
				let start = index * 2; let chunk = digits.get(start..start + 2).ok_or_else(|| {
					return Self::invalid_value(path, "hex byte boundary is not canonical ASCII");
				})?; return u8::from_str_radix(chunk, 16).map_err(|error| { return decode_error(
						CheckpointDecodeErrorKind::InvalidValue, path.clone(),
						format!("invalid hex byte: {error}"),
					); }); }) .collect(); }

	/// Decode an exact-count canonical hexadecimal u32 image.
	fn parse_u32_hex(&mut self, value: &str, count: usize, path: &CheckpointPath) -> CheckpointResult<Vec<u32>> {
		let digits = canonical_hex(value, path)?; let expected = count .checked_mul(8)
			.ok_or_else(|| return Self::limit(path, "u32 hex digit count overflowed"))?;
		if digits.len() != expected { return Err(Self::invalid_value( path,
				format!("u32 image has {} digits, expected {expected}", digits.len()),
			)); }
		self.reserve_payload( count.checked_mul(4)
				.ok_or_else(|| return Self::limit(path, "u32 payload overflowed"))?,
			path, )?; return (0..count) .map(|index| { let start = index * 8; let chunk = digits .get(start..start + 8)
					.ok_or_else(|| return Self::invalid_value(path, "u32 boundary is not canonical ASCII"))?;
				return u32::from_str_radix(chunk, 16) .map_err(|error| return Self::invalid_value(path, error.to_string())); })
			.collect(); }

	/// Decode an exact-count canonical hexadecimal i32 image.
	fn parse_i32_hex(&mut self, value: &str, count: usize, path: &CheckpointPath) -> CheckpointResult<Vec<i32>> {
		return self .parse_u32_hex(value, count, path) .map(|encoded_values| { return encoded_values .into_iter()
					.map(|encoded| return encoded.cast_signed()) .collect(); }); }

	/// Decode an exact-count canonical hexadecimal usize image.
	fn parse_usize_hex(&mut self, value: &str, count: usize, path: &CheckpointPath) -> CheckpointResult<Vec<usize>> {
		let digits = canonical_hex(value, path)?; let expected = count .checked_mul(16)
			.ok_or_else(|| return Self::limit(path, "usize hex digit count overflowed"))?;
		if digits.len() != expected { return Err(Self::invalid_value( path, format!(
					"source-row image has {} digits, expected {expected}",
					digits.len() ), )); }
		self.reserve_payload( count.checked_mul(8)
				.ok_or_else(|| return Self::limit(path, "source-row payload overflowed"))?,
			path, )?; return (0..count) .map(|index| { let start = index * 16;
				let chunk = digits.get(start..start + 16).ok_or_else(|| {
					return Self::invalid_value(path, "source-row boundary is not canonical ASCII");
				})?; let source_row = u64::from_str_radix(chunk, 16)
					.map_err(|error| return Self::invalid_value(path, error.to_string()))?;
				return usize::try_from(source_row).map_err(|error| {
					return Self::invalid_value(path, format!("source row does not fit usize: {error}"));
				}); }) .collect(); }

	/// Decode an exact-count canonical binary known-value mask.
	fn parse_known(&mut self, value: &str, count: usize, path: &CheckpointPath) -> CheckpointResult<Vec<i32>> {
		let digits = value
			.strip_prefix("0b")
			.ok_or_else(|| return Self::invalid_value(path, "known mask lacks canonical 0b prefix"))?;
		if digits.len() != count || digits .bytes()
				.any(|byte| return !matches!(byte, b'0' | b'1'))
		{ return Err(Self::invalid_value( path,
				format!("known mask must contain exactly {count} binary digits"),
			)); }
		self.reserve_payload(count, path)?; return Ok(digits .bytes()
			.map(|byte| return i32::from(byte == b'1'))
			.collect()); }

	/// Decode one canonical usize scalar.
	fn parse_usize(value: &str, path: &CheckpointPath) -> CheckpointResult<usize> { return parse_canonical(value, path); }

	/// Decode one canonical u32 scalar.
	fn parse_u32(value: &str, path: &CheckpointPath) -> CheckpointResult<u32> { return parse_canonical(value, path); }

	/// Decode an optional canonical u8 scalar.
	fn parse_optional_u8(value: &str, path: &CheckpointPath) -> CheckpointResult<Option<u8>> {
		return if value == "none" {
			Ok(None) } else { parse_canonical(value, path).map(Some) }; }

	/// Require an exact scalar value.
	fn expect(actual: &str, expected: &str, path: &CheckpointPath) -> CheckpointResult<()> { return if actual == expected {
			Ok(()) } else { Err(Self::invalid_value( path,
				format!("expected {expected:?}, found {actual:?}"),
			)) }; }

	/// Construct an invalid-value decode error.
	fn invalid_value(path: &CheckpointPath, detail: impl Into<String>) -> CheckpointError { return decode_error(
			CheckpointDecodeErrorKind::InvalidValue, path.clone(), detail, ); }

	/// Construct an unknown-field decode error.
	fn unknown(path: &CheckpointPath, detail: impl Into<String>) -> CheckpointError { return decode_error(
			CheckpointDecodeErrorKind::UnknownField, path.clone(), detail, ); }

	/// Construct a decode-limit error.
	fn limit(path: &CheckpointPath, detail: impl Into<String>) -> CheckpointError { return decode_error(
			CheckpointDecodeErrorKind::LimitExceeded, path.clone(), detail, ); } }

/// Validate and return canonical lowercase hexadecimal digits.
fn canonical_hex<'a>(value: &'a str, path: &CheckpointPath) -> CheckpointResult<&'a str> {
	let digits = value.strip_prefix("0x").ok_or_else(|| {
		return decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
			"hex value lacks canonical lowercase 0x prefix",
		); })?; if digits .bytes() .any(|byte| return !byte.is_ascii_hexdigit() || byte.is_ascii_uppercase())
	{ return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
			"hex value contains a noncanonical digit",
		)); }
	return Ok(digits); }

/// Decode one canonical decimal integer.
fn parse_canonical<T>(value: &str, path: &CheckpointPath) -> CheckpointResult<T>
where T: str::FromStr + ToString, T::Err: fmt::Display, { let parsed = value.parse::<T>().map_err(|error| {
		return decode_error( CheckpointDecodeErrorKind::InvalidValue, path.clone(),
			format!("invalid integer {value:?}: {error}"),
		); })?; if parsed.to_string() != value { return Err(decode_error( CheckpointDecodeErrorKind::InvalidValue,
			path.clone(),
			format!("integer {value:?} is not canonical"),
		)); }
	return Ok(parsed); }
