use core::fmt; use alloc::collections::{BTreeMap, BTreeSet}; use std::{ path::Path, };

use recipe_core::{DType, ScalarOpcode, ValueId}; use recipe_ingest::{
	GgufArchive, GgufEndian, GgufLimits, GgufMetadataValue, GgufTensorType, RawTable, SourceLimit,
	parse_contract_i32, parse_gguf, read_source_snapshot, };
use recipe_language::{Contraction, Gather, IndexBounds, PrimitiveKind, ScalarProgramBuilder};
use recipe_ops::{PreparedParameter, PreparedParameters};

use super::{ InferenceCompileError, InferenceCompileErrorKind, InferenceCompileResult, InferenceGraphCompiler,
	InferenceInputRole, InferencePredictionKind, InferencePreparationResult, InferenceTask, checked_i32,
	checked_product, shape, }; use crate::{ CompiledInference, Activation, MAXIMUM_REDUCTION_TREE_LANES,
	forward::{add_program, forbidden_aliases, multiply_constant_program, multiply_program}, };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum GgufLlamaErrorKind { Container, UnsupportedArchitecture, UnsupportedVariant, MissingMetadata, InvalidMetadata,
	MissingTensor, InvalidTensor, InvalidTokenStream, ArithmeticOverflow, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GgufLlamaError { kind: GgufLlamaErrorKind, detail: String, }

impl GgufLlamaError {
	#[must_use]
	#[inline]
	pub const fn kind(&self) -> GgufLlamaErrorKind { self.kind }

	#[must_use]
	#[inline]
	pub fn detail(&self) -> &str { &self.detail }

	fn new(kind: GgufLlamaErrorKind, detail: impl Into<String>) -> Self { Self { kind, detail: detail.into(), } } }

impl fmt::Display for GgufLlamaError {
	#[inline]
	fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(formatter, "{:?}: {}", self.kind, self.detail)
	} }

impl core::error::Error for GgufLlamaError {}

pub type GgufLlamaResult<T> = Result<T, GgufLlamaError>;

#[derive(Clone, Debug, PartialEq, Eq)]
struct GgufLlamaTensorImage { name: String, shape: Vec<u64>, bytes: Vec<u8>, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GgufLlamaLinear { weight: usize, bias: Option<usize>, scale: Option<usize>, }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct GgufLlamaBlock { attention_norm: usize, query: GgufLlamaLinear, key: GgufLlamaLinear, value: GgufLlamaLinear,
	attention_output: GgufLlamaLinear, feed_forward_norm: usize, feed_forward_gate: GgufLlamaLinear,
	feed_forward_up: GgufLlamaLinear, feed_forward_down: GgufLlamaLinear, }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GgufLlamaArtifact { vocabulary: u64, context_length: u64, embedding_length: u64, feed_forward_length: u64,
	heads: u64, head_dimension: u64, rms_epsilon_bits: u32, rope_base_bits: u32, rope_frequency_scale_bits: u32,
	rope_attention_factor_bits: u32, attention_scale_bits: u32, clamp_kqv_bits: u32, tensors: Vec<GgufLlamaTensorImage>,
	token_embedding: usize, output_norm: usize, output: usize, rope_factors: Option<usize>, blocks: Vec<GgufLlamaBlock>, }

impl GgufLlamaArtifact {
	#[must_use]
	#[inline]
	pub const fn architecture(&self) -> &'static str { "llama" }

	#[must_use]
	#[inline]
	pub const fn vocabulary(&self) -> u64 { self.vocabulary }

	#[must_use]
	#[inline]
	pub const fn context_length(&self) -> u64 { self.context_length }

	#[must_use]
	#[inline]
	pub const fn embedding_length(&self) -> u64 { self.embedding_length }

	#[must_use]
	#[inline]
	pub fn block_count(&self) -> usize { self.blocks.len() }

	fn tensor(&self, index: usize) -> &GgufLlamaTensorImage { &self.tensors[index] }

	fn execution_tensor_indices(&self) -> BTreeSet<usize> {
		let mut indices = BTreeSet::from([self.token_embedding, self.output_norm, self.output]); for block in &self.blocks {
			indices.insert(block.attention_norm); indices.insert(block.feed_forward_norm); for linear in [ block.query,
				block.key, block.value, block.attention_output, block.feed_forward_gate, block.feed_forward_up,
				block.feed_forward_down, ] { indices.insert(linear.weight); indices.extend(linear.bias);
				indices.extend(linear.scale); } }
		if let Some(rope_factors) = self.rope_factors { indices.remove(&rope_factors); }
		indices } }

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedGgufLlamaInference { artifact: GgufLlamaArtifact, tokens: Vec<i32>, }

impl PreparedGgufLlamaInference {
	#[must_use]
	#[inline]
	pub const fn artifact(&self) -> &GgufLlamaArtifact { &self.artifact }

	#[must_use]
	#[inline]
	pub fn tokens(&self) -> &[i32] { &self.tokens } }

#[inline]
pub fn decode_gguf_llama(bytes: &[u8], limits: GgufLimits) -> GgufLlamaResult<GgufLlamaArtifact> {
	let archive = parse_gguf(bytes, limits)
		.map_err(|error| GgufLlamaError::new(GgufLlamaErrorKind::Container, error.to_string()))?; if archive.version() != 3 {
		return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant, format!(
				"GGUF version {} is not the supported v3 container",
				archive.version() ), )); }
	if archive.endian() != GgufEndian::Little { return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant,
			"the first llama execution instrument requires little-endian F32 tensor images",
		)); }
	let architecture = metadata_string(&archive, "general.architecture")?;
	if architecture != "llama" {
		return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedArchitecture,
			format!("GGUF architecture {architecture:?} has no Recipe execution instrument"),
		)); }

	let vocabulary = metadata_u32(&archive, "llama.vocab_size")?;
	let context_length = metadata_u32(&archive, "llama.context_length")?;
	let embedding_length = metadata_u32(&archive, "llama.embedding_length")?;
	let block_count = metadata_u32(&archive, "llama.block_count")?;
	let feed_forward_length = metadata_u32(&archive, "llama.feed_forward_length")?;
	let heads = metadata_u32(&archive, "llama.attention.head_count")?;
	let key_value_heads = metadata_u32(&archive, "llama.attention.head_count_kv")?;
	let rotary_dimension = metadata_u32(&archive, "llama.rope.dimension_count")?;
	for (name, value) in [
		("vocabulary", vocabulary),
		("context length", context_length),
		("embedding length", embedding_length),
		("block count", block_count),
		("feed-forward length", feed_forward_length),
		("attention head count", heads),
	] { if value == 0 { return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidMetadata,
				format!("llama {name} must be nonzero"),
			)); } }
	if vocabulary > i32::MAX as u64 { return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant,
			"the llama vocabulary exceeds Recipe's exact int32 token domain",
		)); }
	if heads != key_value_heads { return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant, format!(
				"grouped-query attention is not in the first llama instrument ({heads} query heads, {key_value_heads} KV heads)"
			), )); }
	if !embedding_length.is_multiple_of(heads) { return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidMetadata,
			"llama embedding length is not evenly divisible by its attention heads",
		)); }
	let head_dimension = embedding_length / heads;
	if rotary_dimension != head_dimension || !rotary_dimension.is_multiple_of(2) { return Err(GgufLlamaError::new(
			GgufLlamaErrorKind::UnsupportedVariant, format!(
				"the first llama instrument requires even full-head RoPE; rotary dimension {rotary_dimension}, head dimension {head_dimension}"
			), )); }
	let key_length = metadata_optional_u32(&archive, "llama.attention.key_length")?.unwrap_or(head_dimension);
	let value_length = metadata_optional_u32(&archive, "llama.attention.value_length")?.unwrap_or(head_dimension);
	if key_length != head_dimension || value_length != head_dimension { return Err(GgufLlamaError::new(
			GgufLlamaErrorKind::UnsupportedVariant,
			"the first llama instrument requires key and value head widths equal to the embedding head width",
		)); }
	if metadata_optional_u32(&archive, "llama.expert_count")?.unwrap_or(0) != 0 {
		return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant,
			"mixture-of-experts llama tensors are not part of the dense llama instrument",
		)); }
	if metadata_optional_bool(&archive, "llama.use_parallel_residual")?.unwrap_or(false) {
		return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant,
			"parallel-residual llama blocks require a distinct execution instrument",
		)); }
	if !metadata_optional_bool(&archive, "llama.attention.causal")?.unwrap_or(true) {
		return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant,
			"non-causal llama attention requires a distinct execution instrument",
		)); }
	let rms_epsilon = metadata_f32(&archive, "llama.attention.layer_norm_rms_epsilon")?;
	if !(rms_epsilon.is_finite() && rms_epsilon > 0.0) { return Err(GgufLlamaError::new(
			GgufLlamaErrorKind::InvalidMetadata,
			"llama RMS normalization epsilon must be finite and positive",
		)); }
	let rope_base = metadata_optional_f32(&archive, "llama.rope.freq_base")?.unwrap_or(10_000.0);
	if !(rope_base.is_finite() && rope_base > 0.0) { return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidMetadata,
			"llama RoPE frequency base must be finite and positive",
		)); }
	let scaling_type = metadata_optional_string(&archive, "llama.rope.scaling.type")?.unwrap_or("linear");
	let scaling_factor = metadata_optional_f32(&archive, "llama.rope.scaling.factor")?.unwrap_or(0.0);
	let rope_frequency_scale = match scaling_type {
		"none" => 1.0,
		"linear" if scaling_factor == 0.0 => 1.0,
		"linear" if scaling_factor.is_finite() && scaling_factor > 0.0 => 1.0 / scaling_factor,
		"linear" => {
			return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidMetadata,
				"nonzero linear RoPE scaling factor must be finite and positive",
			)); }
		other => { return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant,
				format!("RoPE scaling type {other:?} requires a distinct llama instrument"),
			)); } };
	let configured_ext = metadata_optional_f32(&archive, "llama.rope.scaling.yarn_ext_factor")?.unwrap_or(-1.0);
	let effective_ext = if configured_ext < 0.0 { 0.0 } else { configured_ext }; if effective_ext != 0.0 {
		return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant,
			"YaRN extrapolation requires a distinct llama RoPE instrument",
		)); }
	let yarn_attention_factor =
		metadata_optional_f32(&archive, "llama.rope.scaling.yarn_attn_factor")?.unwrap_or(1.0);
	let rope_attention_factor = metadata_optional_f32(&archive, "llama.rope.scaling.attn_factor")?.unwrap_or(1.0);
	let effective_attention_factor = yarn_attention_factor * rope_attention_factor;
	if !(effective_attention_factor.is_finite() && effective_attention_factor > 0.0) { return Err(GgufLlamaError::new(
			GgufLlamaErrorKind::InvalidMetadata,
			"effective llama RoPE attention factor must be finite and positive",
		)); }
	let configured_attention_scale = metadata_optional_f32(&archive, "llama.attention.scale")?.unwrap_or(0.0);
	let attention_scale = if configured_attention_scale == 0.0 { 1.0 / (head_dimension as f32).sqrt() } else {
		configured_attention_scale }; if !(attention_scale.is_finite() && attention_scale > 0.0) {
		return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidMetadata,
			"llama attention score scale must be finite and positive",
		)); }
	let clamp_kqv = metadata_optional_f32(&archive, "llama.attention.clamp_kqv")?.unwrap_or(0.0);
	if !(clamp_kqv.is_finite() && clamp_kqv >= 0.0) { return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidMetadata,
			"llama Q/K/V clamp must be finite and nonnegative",
		)); }
	require_zero_swiglu_clamps(&archive, block_count)?;

	let mut tensors = ArtifactTensorBuilder::new(&archive);
	let token_embedding = tensors.required("token_embd.weight", &[embedding_length, vocabulary])?;
	let output_norm = tensors.required("output_norm.weight", &[embedding_length])?;
	let output = tensors
		.optional("output.weight", &[embedding_length, vocabulary])?
		.unwrap_or(token_embedding);
	let rope_factors = tensors.optional("rope_freqs.weight", &[rotary_dimension / 2])?;
	let mut blocks = Vec::with_capacity(to_usize(block_count, "llama block count")?);
	for block in 0..block_count {
		let prefix = format!("blk.{block}");
		let attention_norm = tensors.required(&format!("{prefix}.attn_norm.weight"), &[embedding_length])?;
		let query = tensors.linear( &prefix,
			"attn_q",
			&[embedding_length, embedding_length], embedding_length, )?; let key = tensors.linear( &prefix,
			"attn_k",
			&[embedding_length, embedding_length], embedding_length, )?; let value = tensors.linear( &prefix,
			"attn_v",
			&[embedding_length, embedding_length], embedding_length, )?; let attention_output = tensors.linear( &prefix,
			"attn_output",
			&[embedding_length, embedding_length], embedding_length, )?;
		let feed_forward_norm = tensors.required(&format!("{prefix}.ffn_norm.weight"), &[embedding_length])?;
		let feed_forward_gate = tensors.linear( &prefix,
			"ffn_gate",
			&[embedding_length, feed_forward_length], feed_forward_length, )?; let feed_forward_up = tensors.linear( &prefix,
			"ffn_up",
			&[embedding_length, feed_forward_length], feed_forward_length, )?; let feed_forward_down = tensors.linear( &prefix,
			"ffn_down",
			&[feed_forward_length, embedding_length], embedding_length, )?; for stem in [
			"attn_q",
			"attn_k",
			"attn_v",
			"attn_output",
			"ffn_gate",
			"ffn_up",
			"ffn_down",
		] {
			tensors.ignore_optional(&format!("{prefix}.{stem}.input_scale"), &[1])?;
		}
		blocks.push(GgufLlamaBlock { attention_norm, query, key, value, attention_output, feed_forward_norm,
			feed_forward_gate, feed_forward_up, feed_forward_down, }); }
	let tensor_images = tensors.finish()?; if let Some(index) = rope_factors {
		for (factor, bytes) in tensor_images[index].bytes.chunks_exact(4).enumerate() {
			let value = f32::from_le_bytes(bytes.try_into().expect("one F32 factor"));
			if !value.is_finite() || value == 0.0 { return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidTensor,
					format!("rope_freqs.weight[{factor}] must be finite and nonzero"),
				)); } } }
	Ok(GgufLlamaArtifact { vocabulary, context_length, embedding_length, feed_forward_length, heads, head_dimension,
		rms_epsilon_bits: rms_epsilon.to_bits(), rope_base_bits: rope_base.to_bits(),
		rope_frequency_scale_bits: rope_frequency_scale.to_bits(),
		rope_attention_factor_bits: effective_attention_factor.to_bits(), attention_scale_bits: attention_scale.to_bits(),
		clamp_kqv_bits: clamp_kqv.to_bits(), tensors: tensor_images, token_embedding, output_norm, output, rope_factors,
		blocks, }) }

#[inline]
pub fn load_gguf_llama_model_file(path: &Path, limits: GgufLimits) -> InferencePreparationResult<GgufLlamaArtifact> {
	let source_limit = SourceLimit::new(limits.file_bytes().get())?;
	let snapshot = read_source_snapshot(path, source_limit)?;
	decode_gguf_llama(snapshot.bytes(), limits).map_err(Into::into) }

#[inline]
pub fn prepare_gguf_llama_inference_table( artifact: GgufLlamaArtifact, table: &RawTable,
) -> InferencePreparationResult<PreparedGgufLlamaInference> { if table.width() != 1 { return Err(GgufLlamaError::new(
			GgufLlamaErrorKind::InvalidTokenStream, format!(
				"GGUF llama inference consumes one ordered token stream, but the selected table has {} vectors",
				table.width() ), ) .into()); }
	let mut tokens = Vec::new(); for (row, fields) in table.rows().iter().enumerate() {
		let Some(field) = fields.first() else { return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidTokenStream,
				format!("token row {row} is empty"),
			) .into()); }; for token in field .split(|byte| byte.is_ascii_whitespace()) .filter(|token| !token.is_empty())
		{ let token = core::str::from_utf8(token).map_err(|error| { GgufLlamaError::new(
					GgufLlamaErrorKind::InvalidTokenStream,
					format!("token row {row} is not UTF-8: {error}"),
				) })?; let value = parse_contract_i32(token) .map_err(|error| { GgufLlamaError::new(
						GgufLlamaErrorKind::InvalidTokenStream,
						format!("token row {row} contains a non-int32 token: {error}"),
					) })? .value(); if value < 0
				|| u64::try_from(value) .ok() .is_none_or(|value| value >= artifact.vocabulary)
			{ return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidTokenStream, format!(
						"token {} is outside the exact vocabulary domain 0..{}",
						value, artifact.vocabulary ), ) .into()); }
			tokens.push(value); } }
	if tokens.is_empty() { return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidTokenStream,
			"GGUF llama inference requires at least one token",
		) .into()); }
	if u64::try_from(tokens.len()) .ok() .is_none_or(|count| count > artifact.context_length)
	{ return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidTokenStream, format!(
				"token stream length {} exceeds model context {}",
				tokens.len(), artifact.context_length ), ) .into()); }
	Ok(PreparedGgufLlamaInference { artifact, tokens }) }

#[inline]
pub fn compile_prepared_gguf_llama_inference( prepared: &PreparedGgufLlamaInference,
) -> InferenceCompileResult<CompiledInference> { let artifact = &prepared.artifact;
	let sequence = u64::try_from(prepared.tokens.len()).map_err(|error| { InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("GGUF llama token count does not fit u64: {error}"),
		) })?; let mut compiler = InferenceGraphCompiler::new(); let token_ids = compiler.external(
		InferenceInputRole::GgufTokenIds, DType::I32, shape(&[sequence])?, prepared .tokens .iter()
			.flat_map(|token| token.to_le_bytes()) .collect(), )?; let mut values = BTreeMap::new();
	for index in artifact.execution_tensor_indices() { let tensor = artifact.tensor(index); let value = compiler.external(
			InferenceInputRole::GgufTensor { tensor: index }, DType::F32, shape(&tensor.shape)?, tensor.bytes.clone(), )?;
		values.insert(index, value); }
	let token_embedding = tensor_value(&values, artifact.token_embedding)?;
	let mut current = compiler.tensor(DType::F32, shape(&[sequence, artifact.embedding_length])?)?; compiler.emit(
		vec![token_embedding, token_ids], vec![current], PrimitiveKind::Gather(Gather { axis: 0, bounds: IndexBounds::Reject,
		}), forbidden_aliases(2, 1), )?; let rope = prepare_rope_inputs(&mut compiler, artifact, sequence)?;
	for block in &artifact.blocks { let residual = current; current = rms_norm( &mut compiler, current,
			tensor_value(&values, block.attention_norm)?, sequence, artifact.embedding_length,
			f32::from_bits(artifact.rms_epsilon_bits), )?; let query = linear( &mut compiler, current, block.query, &values,
			[sequence, artifact.embedding_length, artifact.embedding_length], true, f32::from_bits(artifact.clamp_kqv_bits), )?;
		let key = linear( &mut compiler, current, block.key, &values,
			[sequence, artifact.embedding_length, artifact.embedding_length], true, f32::from_bits(artifact.clamp_kqv_bits), )?;
		let value = linear( &mut compiler, current, block.value, &values,
			[sequence, artifact.embedding_length, artifact.embedding_length], true, f32::from_bits(artifact.clamp_kqv_bits), )?;
		let attention = causal_attention(&mut compiler, artifact, sequence, query, key, value, &rope)?; current = linear(
			&mut compiler, attention, block.attention_output, &values,
			[sequence, artifact.embedding_length, artifact.embedding_length], true, 0.0, )?;
		current = compiler.exact_add(current, residual)?; let feed_forward_residual = current; current = rms_norm(
			&mut compiler, current, tensor_value(&values, block.feed_forward_norm)?, sequence, artifact.embedding_length,
			f32::from_bits(artifact.rms_epsilon_bits), )?; let gate = linear( &mut compiler, current, block.feed_forward_gate,
			&values, [sequence, artifact.embedding_length, artifact.feed_forward_length], false, 0.0, )?; let up = linear(
			&mut compiler, current, block.feed_forward_up, &values,
			[sequence, artifact.embedding_length, artifact.feed_forward_length], false, 0.0, )?;
		let gate = compiler.apply_activation( gate, Activation::Silu, None,
			shape(&[sequence, artifact.feed_forward_length])?, )?; let gated = compiler.tensor( DType::F32,
			shape(&[sequence, artifact.feed_forward_length])?, )?;
		compiler.emit_elementwise(vec![gate, up], vec![gated], multiply_program()?)?; current = linear( &mut compiler, gated,
			block.feed_forward_down, &values, [sequence, artifact.feed_forward_length, artifact.embedding_length], false, 0.0,
		)?; current = compiler.exact_add(current, feed_forward_residual)?; }
	current = rms_norm( &mut compiler, current, tensor_value(&values, artifact.output_norm)?, sequence,
		artifact.embedding_length, f32::from_bits(artifact.rms_epsilon_bits), )?;
	let output_weight = tensor_value(&values, artifact.output)?;
	let logits = compiler.tensor(DType::F32, shape(&[sequence, artifact.vocabulary])?)?; compiler.emit(
		vec![current, output_weight], vec![logits], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(),
			contract_axes: vec![(1, 1)], }), forbidden_aliases(2, 1), )?; compiler.finish( logits,
		InferencePredictionKind::TokenLogits, sequence, InferenceTask::TokenLogits { vocabulary: artifact.vocabulary, },
		vec![DType::I32], None, ) }

#[derive(Clone, Copy, Debug)]
struct RopeInputs { partners: ValueId, cosines: ValueId, signed_sines: ValueId, }

fn prepare_rope_inputs( compiler: &mut InferenceGraphCompiler, artifact: &GgufLlamaArtifact, sequence: u64,
) -> InferenceCompileResult<RopeInputs> { let rotary_half = artifact.head_dimension / 2;
	let factor_bytes = match artifact.rope_factors { Some(index) => artifact.tensor(index).bytes.clone(),
		None => (0..rotary_half) .flat_map(|_| 1.0f32.to_le_bytes()) .collect(), }; let factor_values = factor_bytes
		.chunks_exact(4)
		.map(|bytes| f32::from_le_bytes(bytes.try_into().expect("one validated RoPE factor")))
		.collect::<Vec<_>>(); let rows = sequence.checked_mul(artifact.heads).ok_or_else(|| { InferenceCompileError::new(
			InferenceCompileErrorKind::ArithmeticOverflow,
			"GGUF llama RoPE row count overflowed u64",
		) })?; let elements = checked_product( &[rows, artifact.head_dimension],
		"GGUF llama RoPE element count",
	)?;
	checked_i32(elements, "GGUF llama RoPE element count")?;
	let capacity = usize::try_from(elements).map_err(|error| { InferenceCompileError::new(
			InferenceCompileErrorKind::UnsupportedExtent,
			format!("GGUF llama RoPE element count does not fit usize: {error}"),
		) })?; let theta_scale = f32::from_bits(artifact.rope_base_bits).powf(-2.0 / artifact.head_dimension as f32);
	let frequency_scale = f32::from_bits(artifact.rope_frequency_scale_bits);
	let attention_factor = f32::from_bits(artifact.rope_attention_factor_bits);
	let mut partner_indices = Vec::with_capacity(capacity); let mut cosine_bits = Vec::with_capacity(capacity);
	let mut signed_sine_bits = Vec::with_capacity(capacity); for row in 0..rows { let token = row / artifact.heads;
		let mut theta = token as f32; for pair in 0..rotary_half {
			let factor = factor_values[usize::try_from(pair).expect("validated RoPE half fits usize")];
			let angle = frequency_scale * theta / factor; let cosine = angle.cos() * attention_factor;
			let sine = angle.sin() * attention_factor; let base = row .checked_mul(artifact.head_dimension)
				.and_then(|base| base.checked_add(pair * 2)) .ok_or_else(|| { InferenceCompileError::new(
						InferenceCompileErrorKind::ArithmeticOverflow,
						"GGUF llama RoPE partner index overflowed u64",
					) })?;
			partner_indices.push(checked_i32(base + 1, "GGUF llama RoPE partner index")?);
			partner_indices.push(checked_i32(base, "GGUF llama RoPE partner index")?);
			cosine_bits.extend([cosine.to_bits(), cosine.to_bits()]);
			signed_sine_bits.extend([(-sine).to_bits(), sine.to_bits()]); theta *= theta_scale; } }
	let flat_shape = shape(&[elements])?; let partners = compiler.external_i32( InferenceInputRole::GgufRopePartnerIndices,
		flat_shape.clone(), &partner_indices, )?; let cosines = compiler.external( InferenceInputRole::GgufRopeCosines,
		DType::F32, flat_shape.clone(), cosine_bits .iter() .flat_map(|bits| bits.to_le_bytes()) .collect(), )?;
	let signed_sines = compiler.external( InferenceInputRole::GgufRopeSignedSines, DType::F32, flat_shape, signed_sine_bits
			.iter() .flat_map(|bits| bits.to_le_bytes()) .collect(), )?; Ok(RopeInputs { partners, cosines, signed_sines, }) }

fn causal_attention( compiler: &mut InferenceGraphCompiler, artifact: &GgufLlamaArtifact, sequence: u64, query: ValueId,
	key: ValueId, value: ValueId, rope: &RopeInputs, ) -> InferenceCompileResult<ValueId> {
	let query = apply_rope(compiler, artifact, sequence, query, rope)?;
	let key = apply_rope(compiler, artifact, sequence, key, rope)?;
	let head_shape = shape(&[1, sequence, artifact.heads, artifact.head_dimension])?;
	let value = compiler.reinterpret_f32(value, head_shape)?;
	let score_shape = shape(&[1, artifact.heads, sequence, sequence])?;
	let scores = compiler.tensor(DType::F32, score_shape.clone())?; compiler.emit( vec![query, key], vec![scores],
		PrimitiveKind::Contraction(Contraction { batch_axes: vec![(0, 0), (2, 2)], contract_axes: vec![(3, 3)], }),
		forbidden_aliases(2, 1), )?; let scaled = compiler.tensor(DType::F32, score_shape)?; compiler.emit_elementwise(
		vec![scores], vec![scaled], multiply_constant_program(f32::from_bits(artifact.attention_scale_bits))?, )?;
	let probabilities = compiler.causal_softmax( scaled, 1, artifact.heads, sequence, MAXIMUM_REDUCTION_TREE_LANES, )?;
	let context = compiler.tensor( DType::F32, shape(&[1, artifact.heads, sequence, artifact.head_dimension])?, )?;
	compiler.emit( vec![probabilities, value], vec![context], PrimitiveKind::Contraction(Contraction {
			batch_axes: vec![(0, 0), (1, 2)], contract_axes: vec![(3, 1)], }), forbidden_aliases(2, 1), )?;
	let context = compiler.head_major_to_sequence( context, 1, sequence, artifact.heads, artifact.head_dimension, )?;
	compiler.reinterpret_f32(context, shape(&[sequence, artifact.embedding_length])?) }

fn apply_rope( compiler: &mut InferenceGraphCompiler, artifact: &GgufLlamaArtifact, sequence: u64, input: ValueId,
	rope: &RopeInputs, ) -> InferenceCompileResult<ValueId> {
	let rows = sequence.checked_mul(artifact.heads).ok_or_else(|| { InferenceCompileError::new(
			InferenceCompileErrorKind::ArithmeticOverflow,
			"GGUF llama RoPE row count overflowed u64",
		) })?; let elements = checked_product( &[rows, artifact.head_dimension],
		"GGUF llama RoPE element count",
	)?; let values = compiler.reinterpret_f32(input, shape(&[elements])?)?;
	let rotated = compiler.tensor(DType::F32, shape(&[elements])?)?; let parameters = [
		("rows".to_owned(), PreparedParameter::U64(rows)),
		(
			"head_dim".to_owned(),
			PreparedParameter::U64(artifact.head_dimension), ), (
			"rotary_dim".to_owned(),
			PreparedParameter::U64(artifact.head_dimension), ), (
			"heads_per_token".to_owned(),
			PreparedParameter::U64(artifact.heads), ), (
			"theta".to_owned(),
			PreparedParameter::F32Bits(artifact.rope_base_bits), ), (
			"rotation_tables_verified".to_owned(),
			PreparedParameter::Bool(true), ), ] .into_iter() .collect::<PreparedParameters>(); compiler.materialize(
		"gpu_rope_partial",
		&[
			("values", values),
			("partner_indices", rope.partners),
			("cosines", rope.cosines),
			("signed_sines", rope.signed_sines),
		],
		&[("rotated", rotated)],
		"values",
		&parameters, )?; compiler.reinterpret_f32( rotated, shape(&[1, sequence, artifact.heads, artifact.head_dimension])?, )
}

fn linear( compiler: &mut InferenceGraphCompiler, input: ValueId, linear: GgufLlamaLinear,
	values: &BTreeMap<usize, ValueId>, dimensions: [u64; 3], scale_before_bias: bool, clamp: f32,
) -> InferenceCompileResult<ValueId> { let [rows, input_width, output_width] = dimensions; compiler.require_tensor(
		input, DType::F32, &[rows, input_width],
		"GGUF llama linear input",
	)?; let weight = tensor_value(values, linear.weight)?; compiler.require_tensor( weight, DType::F32,
		&[output_width, input_width],
		"GGUF llama linear weight",
	)?; let output_shape = shape(&[rows, output_width])?;
	let mut current = compiler.tensor(DType::F32, output_shape.clone())?; compiler.emit( vec![input, weight],
		vec![current], PrimitiveKind::Contraction(Contraction { batch_axes: Vec::new(), contract_axes: vec![(1, 1)], }),
		forbidden_aliases(2, 1), )?; if scale_before_bias {
		current = apply_optional_scale(compiler, current, linear.scale, values, &output_shape)?; }
	if let Some(bias) = linear.bias { let bias = tensor_value(values, bias)?;
		compiler.require_tensor(bias, DType::F32, &[output_width], "GGUF llama linear bias")?;
		let output = compiler.tensor(DType::F32, output_shape.clone())?;
		compiler.emit_elementwise(vec![current, bias], vec![output], add_program()?)?; current = output; }
	if !scale_before_bias { current = apply_optional_scale(compiler, current, linear.scale, values, &output_shape)?; }
	if clamp > 0.0 { let output = compiler.tensor(DType::F32, output_shape)?;
		compiler.emit_elementwise(vec![current], vec![output], symmetric_clamp_program(clamp)?)?; current = output; }
	Ok(current) }

fn apply_optional_scale( compiler: &mut InferenceGraphCompiler, input: ValueId, scale_index: Option<usize>,
	values: &BTreeMap<usize, ValueId>, output_shape: &recipe_language::Shape, ) -> InferenceCompileResult<ValueId> {
	let Some(scale_index) = scale_index else { return Ok(input); }; let scale = tensor_value(values, scale_index)?;
	compiler.require_tensor(scale, DType::F32, &[1], "GGUF llama linear scale")?;
	let output = compiler.tensor(DType::F32, output_shape.clone())?;
	compiler.emit_elementwise(vec![input, scale], vec![output], multiply_program()?)?; Ok(output) }

fn rms_norm( compiler: &mut InferenceGraphCompiler, input: ValueId, scale: ValueId, rows: u64, columns: u64,
	epsilon: f32, ) -> InferenceCompileResult<ValueId> { compiler.require_tensor( input, DType::F32, &[rows, columns],
		"GGUF llama RMSNorm values",
	)?;
	compiler.require_tensor(scale, DType::F32, &[columns], "GGUF llama RMSNorm scale")?;
	let output = compiler.tensor(DType::F32, shape(&[rows, columns])?)?; let parameters = [ (
			"epsilon".to_owned(),
			PreparedParameter::F32Bits(epsilon.to_bits()), ), (
			"tree_lanes".to_owned(),
			PreparedParameter::U64(u64::from(MAXIMUM_REDUCTION_TREE_LANES)), ), ] .into_iter() .collect::<PreparedParameters>();
	compiler.materialize(
		"gpu_rmsnorm",
		&[("values", input), ("scale", scale)],
		&[("normalized", output)],
		"values",
		&parameters, )?; Ok(output) }

fn symmetric_clamp_program(limit: f32) -> InferenceCompileResult<recipe_core::ScalarProgram> {
	let mut builder = ScalarProgramBuilder::new()?; let value = builder.input(DType::F32)?;
	let lower = builder.f32(-limit)?; let upper = builder.f32(limit)?;
	let value = builder.binary(ScalarOpcode::Maximum, value, lower)?;
	let value = builder.binary(ScalarOpcode::Minimum, value, upper)?; Ok(builder.finish(&[value])?) }

fn tensor_value(values: &BTreeMap<usize, ValueId>, index: usize) -> InferenceCompileResult<ValueId> {
	values.get(&index).copied().ok_or_else(|| { InferenceCompileError::new(
			InferenceCompileErrorKind::InconsistentCheckpoint,
			format!("GGUF llama tensor index {index} was not admitted to the graph boundary"),
		) }) }

struct ArtifactTensorBuilder<'archive, 'data> {
	archive: &'archive GgufArchive<'data>,
	images: Vec<GgufLlamaTensorImage>, consumed: BTreeSet<String>, }

impl<'archive, 'data> ArtifactTensorBuilder<'archive, 'data> {
	fn new(archive: &'archive GgufArchive<'data>) -> Self {
		Self { archive, images: Vec::new(), consumed: BTreeSet::new(), } }

	fn required(&mut self, name: &str, dimensions: &[u64]) -> GgufLlamaResult<usize> {
		self.capture(name, dimensions)?.ok_or_else(|| { GgufLlamaError::new( GgufLlamaErrorKind::MissingTensor,
				format!("required llama tensor {name:?} is absent"),
			) }) }

	fn optional(&mut self, name: &str, dimensions: &[u64]) -> GgufLlamaResult<Option<usize>> { self.capture(name, dimensions) }

	fn ignore_optional(&mut self, name: &str, dimensions: &[u64]) -> GgufLlamaResult<()> {
		if self.archive.tensor(name).is_some() { self.validate(name, dimensions)?; self.consumed.insert(name.to_owned()); }
		Ok(()) }

	fn linear( &mut self, prefix: &str, stem: &str, weight_dimensions: &[u64], output_width: u64,
	) -> GgufLlamaResult<GgufLlamaLinear> { Ok(GgufLlamaLinear {
			weight: self.required(&format!("{prefix}.{stem}.weight"), weight_dimensions)?,
			bias: self.optional(&format!("{prefix}.{stem}.bias"), &[output_width])?,
			scale: self.optional(&format!("{prefix}.{stem}.scale"), &[1])?,
		}) }

	fn capture(&mut self, name: &str, dimensions: &[u64]) -> GgufLlamaResult<Option<usize>> {
		let Some(tensor) = self.archive.tensor(name) else { return Ok(None); }; self.validate(name, dimensions)?;
		let bytes = self.archive.raw_tensor(name).ok_or_else(|| { GgufLlamaError::new( GgufLlamaErrorKind::InvalidTensor,
				format!("validated tensor {name:?} has no encoded byte span"),
			) })?; let index = self.images.len(); self.images.push(GgufLlamaTensorImage { name: name.to_owned(),
			shape: tensor.dimensions().iter().rev().copied().collect(), bytes: bytes.to_vec(), });
		self.consumed.insert(name.to_owned()); Ok(Some(index)) }

	fn validate(&self, name: &str, dimensions: &[u64]) -> GgufLlamaResult<()> { let tensor = self .archive .tensor(name)
			.expect("caller established tensor existence");
		if tensor.tensor_type() != GgufTensorType::F32 || tensor.dimensions() != dimensions { return Err(GgufLlamaError::new(
				GgufLlamaErrorKind::InvalidTensor, format!(
					"llama tensor {name:?} has {:?} dimensions {:?}, expected F32 dimensions {dimensions:?}",
					tensor.tensor_type(), tensor.dimensions() ), )); }
		Ok(()) }

	fn finish(self) -> GgufLlamaResult<Vec<GgufLlamaTensorImage>> { if let Some(tensor) = self .archive .tensors() .iter()
			.find(|tensor| !self.consumed.contains(tensor.name()))
		{ return Err(GgufLlamaError::new( GgufLlamaErrorKind::UnsupportedVariant, format!(
					"llama tensor {:?} has no meaning in the dense-F32 execution instrument",
					tensor.name() ), )); }
		Ok(self.images) } }

fn metadata_value<'archive, 'data>(
	archive: &'archive GgufArchive<'data>,
	key: &str,
) -> GgufLlamaResult<&'archive GgufMetadataValue<'data>> {
	archive .metadata_entry(key) .map(|entry| entry.value()) .ok_or_else(|| { GgufLlamaError::new(
				GgufLlamaErrorKind::MissingMetadata,
				format!("required GGUF metadata {key:?} is absent"),
			) }) }

fn metadata_string<'archive, 'data>(archive: &'archive GgufArchive<'data>, key: &str) -> GgufLlamaResult<&'data str> {
	match metadata_value(archive, key)? { GgufMetadataValue::String(value) => Ok(value),
		value => Err(metadata_type_error(key, "string", value)),
	} }

fn metadata_optional_string<'archive, 'data>(
	archive: &'archive GgufArchive<'data>,
	key: &str,
) -> GgufLlamaResult<Option<&'data str>> {
	match archive.metadata_entry(key).map(|entry| entry.value()) { None => Ok(None),
		Some(GgufMetadataValue::String(value)) => Ok(Some(value)),
		Some(value) => Err(metadata_type_error(key, "string", value)),
	} }

fn metadata_u32(archive: &GgufArchive<'_>, key: &str) -> GgufLlamaResult<u64> {
	match metadata_value(archive, key)? { GgufMetadataValue::U32(value) => Ok(u64::from(*value)),
		value => Err(metadata_type_error(key, "u32", value)),
	} }

fn metadata_optional_u32(archive: &GgufArchive<'_>, key: &str) -> GgufLlamaResult<Option<u64>> {
	match archive.metadata_entry(key).map(|entry| entry.value()) { None => Ok(None),
		Some(GgufMetadataValue::U32(value)) => Ok(Some(u64::from(*value))),
		Some(value) => Err(metadata_type_error(key, "u32", value)),
	} }

fn metadata_f32(archive: &GgufArchive<'_>, key: &str) -> GgufLlamaResult<f32> {
	match metadata_value(archive, key)? { GgufMetadataValue::F32Bits(bits) => Ok(f32::from_bits(*bits)),
		value => Err(metadata_type_error(key, "f32", value)),
	} }

fn metadata_optional_f32(archive: &GgufArchive<'_>, key: &str) -> GgufLlamaResult<Option<f32>> {
	match archive.metadata_entry(key).map(|entry| entry.value()) { None => Ok(None),
		Some(GgufMetadataValue::F32Bits(bits)) => Ok(Some(f32::from_bits(*bits))),
		Some(value) => Err(metadata_type_error(key, "f32", value)),
	} }

fn metadata_optional_bool(archive: &GgufArchive<'_>, key: &str) -> GgufLlamaResult<Option<bool>> {
	match archive.metadata_entry(key).map(|entry| entry.value()) { None => Ok(None),
		Some(GgufMetadataValue::Bool(value)) => Ok(Some(*value)),
		Some(value) => Err(metadata_type_error(key, "bool", value)),
	} }

fn metadata_type_error(key: &str, expected: &str, value: &GgufMetadataValue<'_>) -> GgufLlamaError {
	GgufLlamaError::new( GgufLlamaErrorKind::InvalidMetadata, format!(
			"GGUF metadata {key:?} has type {:?}, expected {expected}",
			value.value_type() ), ) }

fn require_zero_swiglu_clamps(archive: &GgufArchive<'_>, block_count: u64) -> GgufLlamaResult<()> {
	let Some(entry) = archive.metadata_entry("llama.swiglu_clamp_shexp") else {
		return Ok(()); }; let GgufMetadataValue::Array(values) = entry.value() else { return Err(metadata_type_error(
			"llama.swiglu_clamp_shexp",
			"f32 array",
			entry.value(), )); };
	let count = to_usize(block_count, "llama block count")?;
	if values.values().len() < count { return Err(GgufLlamaError::new( GgufLlamaErrorKind::InvalidMetadata,
			"llama SwiGLU clamp array is shorter than block count",
		)); }
	for (block, value) in values.values().iter().take(count).enumerate() {
		let GgufMetadataValue::F32Bits(bits) = value else { return Err(metadata_type_error(
				"llama.swiglu_clamp_shexp",
				"f32 array",
				value, )); }; if f32::from_bits(*bits).abs() > 1.0e-6 { return Err(GgufLlamaError::new(
				GgufLlamaErrorKind::UnsupportedVariant,
				format!("limited SwiGLU in block {block} requires a distinct llama instrument"),
			)); } }
	Ok(()) }

fn to_usize(value: u64, role: &str) -> GgufLlamaResult<usize> { usize::try_from(value).map_err(|error| {
		GgufLlamaError::new( GgufLlamaErrorKind::ArithmeticOverflow,
			format!("{role} does not fit host address space: {error}"),
		) }) }
