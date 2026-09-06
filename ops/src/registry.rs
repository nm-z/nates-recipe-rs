use recipe_core::DType;

use crate::{CompositionRecipe, NonCalculationRecipe, OperationError, OperationErrorKind, OperationResult, PrimitiveRecipe, ScalarRecipe, WorkspaceFormula};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OperationId {
	ordinal: usize,
	surface_line: usize,
	occurrence: u16,
	occurrences: u16,
}

impl OperationId {
	#[must_use]
	pub const fn ordinal(self) -> usize { self.ordinal }

	#[must_use]
	pub const fn surface_line(self) -> usize { self.surface_line }

	#[must_use]
	pub const fn occurrence(self) -> u16 { self.occurrence }

	#[must_use]
	pub const fn occurrences(self) -> u16 { self.occurrences }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyDType {
	UntypedDeviceBuffer,
	F16,
	F32,
	F64,
	U8,
	DynamicQuantized,
	HostBytes,
	HostText,
	HostObject,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonicalDTypeContract {
	Exact {
		inputs: &'static [DType],
		outputs: &'static [DType],
	},
	F32Payload,
	I32Payload,
	F32AndI32Payloads,
	F32OrI32Payload,
	NonNumericHostData,
	LegacyExcluded {
		legacy: LegacyDType,
		canonical_replacement: DType,
	},
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationFamily {
	Facade,
	Parsing,
	Encoding,
	Inference,
	Elementwise,
	Creation,
	ShapeAndIndexing,
	Reduction,
	Scan,
	Histogram,
	Statistics,
	Metric,
	Loss,
	Distance,
	Contraction,
	Solver,
	Fft,
	Convolution,
	Pooling,
	Activation,
	Normalization,
	Embedding,
	Attention,
	Recurrent,
	Sequence,
	StateSpace,
	MixtureOfExperts,
	Optimizer,
	Quantization,
	Diffusion,
	Graph,
	Clustering,
	Bayesian,
	SupportVectorMachine,
	ReinforcementLearning,
	Tree,
	Random,
	Workspace,
	Lifecycle,
	HostBehavior,
	Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsupportedReason {
	DedicatedPrimitiveCompositionPending,
	HostBehaviorPending,
	LegacyDTypeExcluded,
	LifecycleOperation,
	WorkspaceQuery,
	DynamicFormatConversionPending,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoweringAvailability {
	Scalar(ScalarRecipe),
	Primitive(PrimitiveRecipe),
	Composition(CompositionRecipe),
	Workspace(WorkspaceFormula),
	NonCalculation(NonCalculationRecipe),
	Unsupported(UnsupportedReason),
}

impl LoweringAvailability {
	#[must_use]
	pub const fn definition(self) -> &'static str {
		match self {
			Self::Scalar(recipe) => recipe.definition(),
			Self::Primitive(recipe) => recipe.definition(),
			Self::Composition(recipe) => recipe.definition(),
			Self::Workspace(formula) => formula.definition(),
			Self::NonCalculation(recipe) => recipe.definition(),
			Self::Unsupported(UnsupportedReason::DedicatedPrimitiveCompositionPending) => "accepted surface operation awaiting a dedicated owned primitive composition",
			Self::Unsupported(UnsupportedReason::HostBehaviorPending) => "host-side public behavior awaiting dependency-clean facade migration",
			Self::Unsupported(UnsupportedReason::LegacyDTypeExcluded) => "legacy non-f32/int32 calculation path excluded by the canonical dtype contract",
			Self::Unsupported(UnsupportedReason::LifecycleOperation) => "lifecycle behavior tracked explicitly and not represented as a calculation",
			Self::Unsupported(UnsupportedReason::WorkspaceQuery) => "static scratch-size query awaiting its owned primitive resource formula",
			Self::Unsupported(UnsupportedReason::DynamicFormatConversionPending) => "dynamic legacy format conversion awaiting an explicit f32/int32 ingestion recipe",
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasContract {
	NoAlias,
	OutputMustAliasInput { input: usize },
	OperationSpecific,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeterminismContract {
	PerElementExactOrder,
	FixedPrimitiveOrder,
	CounterBasedRandom,
	ExplicitAtomicPolicy,
	PendingDefinition,
	HostDeterministic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperationDescriptor {
	pub id: OperationId,
	pub symbol: &'static str,
	pub source: &'static str,
	pub family: OperationFamily,
	pub dtypes: CanonicalDTypeContract,
	pub lowering: LoweringAvailability,
	pub definition: &'static str,
	pub alias: AliasContract,
	pub determinism: DeterminismContract,
	/// Legacy calculation dtype removed at the compatibility boundary. When
	/// present, `dtypes` and `lowering` describe the canonical f32/int32
	/// replacement rather than authorizing the legacy payload type.
	pub legacy_dtype: Option<LegacyDType>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct RawSurfaceEntry {
	pub ordinal: usize,
	pub line: usize,
	pub symbol: &'static str,
	pub source: &'static str,
	pub occurrence: u16,
	pub occurrences: u16,
}

include!(concat!(env!("OUT_DIR"), "/operation_surface.rs"));

const RECIPE_OWNED_OPERATIONS: &[RawSurfaceEntry] = &[
	RawSurfaceEntry {
		ordinal: RAW_OPERATION_COUNT,
		line: 0,
		symbol: "recipe_max_pool_1d",
		source: "ops/src/pooling.rs:channelwise_max_pool_1d",
		occurrence: 1,
		occurrences: 1,
	},
	RawSurfaceEntry {
		ordinal: RAW_OPERATION_COUNT + 1,
		line: 0,
		symbol: "recipe_max_pool_1d_backward",
		source: "ops/src/pooling.rs:channelwise_max_pool_1d_backward",
		occurrence: 1,
		occurrences: 1,
	},
];

#[derive(Clone, Copy, Debug, Default)]
pub struct OperationRegistry;

#[must_use]
pub const fn operation_registry() -> OperationRegistry { OperationRegistry }

impl OperationRegistry {
	/// Total canonical descriptors: the immutable legacy surface prefix followed
	/// by Recipe-owned extensions.
	#[must_use]
	pub const fn len(self) -> usize { RAW_OPERATION_COUNT + RECIPE_OWNED_OPERATIONS.len() }

	#[must_use]
	pub const fn is_empty(self) -> bool { self.len() == 0 }

	/// Iterate the complete canonical registry without changing legacy ordinal
	/// order. Recipe-owned extensions always follow the surface prefix.
	pub fn iter(self) -> impl ExactSizeIterator<Item = OperationDescriptor> {
		(0..self.len()).map(|ordinal| {
			match ordinal < RAW_OPERATION_COUNT {
				true => describe(&RAW_OPERATION_SURFACE[ordinal]),
				false => describe(&RECIPE_OWNED_OPERATIONS[ordinal - RAW_OPERATION_COUNT]),
			}
		})
	}

	pub fn named(self, symbol: &str) -> impl Iterator<Item = OperationDescriptor> + '_ {
		self.iter()
			.filter(move |descriptor| descriptor.symbol == symbol)
	}

	pub fn resolve_unique(self, symbol: &str) -> OperationResult<OperationDescriptor> {
		let mut matches = self.named(symbol);
		let first = matches.next().ok_or_else(|| {
			OperationError::new(
				OperationErrorKind::UnknownOperation,
				format!("operation symbol {symbol:?} is absent"),
			)
		})?;
		if matches.next().is_some() {
			let count = self.named(symbol).count();
			return Err(OperationError::new(
				OperationErrorKind::AmbiguousSymbol,
				format!("operation symbol {symbol:?} has {count} source-qualified entries"),
			));
		}
		Ok(first)
	}

	pub fn resolve_exact(self, symbol: &str, source: &str) -> OperationResult<OperationDescriptor> {
		let mut matches = self
			.iter()
			.filter(|descriptor| descriptor.symbol == symbol && descriptor.source == source);
		let descriptor = matches.next().ok_or_else(|| {
			OperationError::new(
				OperationErrorKind::UnknownOperation,
				format!("operation ({symbol:?}, {source:?}) is absent from the canonical registry"),
			)
		})?;
		if matches.next().is_some() {
			return Err(OperationError::new(
				OperationErrorKind::AmbiguousSymbol,
				format!("operation ({symbol:?}, {source:?}) occurs more than once"),
			));
		}
		Ok(descriptor)
	}
}

fn describe(raw: &RawSurfaceEntry) -> OperationDescriptor {
	let lowering = lowering(raw.symbol, raw.source);
	OperationDescriptor {
		id: OperationId {
			ordinal: raw.ordinal,
			surface_line: raw.line,
			occurrence: raw.occurrence,
			occurrences: raw.occurrences,
		},
		symbol: raw.symbol,
		source: raw.source,
		family: family(raw.symbol, raw.source, lowering),
		dtypes: dtype_contract(raw.symbol, raw.source, lowering),
		lowering,
		definition: lowering.definition(),
		alias: alias_contract(raw.symbol),
		determinism: determinism(raw.symbol, lowering),
		legacy_dtype: explicit_legacy_dtype(raw.symbol, raw.source),
	}
}

fn lowering(symbol: &str, source: &str) -> LoweringAvailability {
	if let Some(recipe) = ScalarRecipe::for_symbol(symbol) {
		return LoweringAvailability::Scalar(recipe);
	}
	if let Some(recipe) = PrimitiveRecipe::for_symbol(symbol) {
		return LoweringAvailability::Primitive(recipe);
	}
	if let Some(formula) = WorkspaceFormula::for_symbol(symbol) {
		return LoweringAvailability::Workspace(formula);
	}
	if let Some(recipe) = NonCalculationRecipe::for_entry(symbol, source) {
		return LoweringAvailability::NonCalculation(recipe);
	}
	if let Some(recipe) = CompositionRecipe::for_entry(symbol, source) {
		return LoweringAvailability::Composition(recipe);
	}
	if explicit_legacy_dtype(symbol, source).is_some() {
		return LoweringAvailability::Unsupported(UnsupportedReason::LegacyDTypeExcluded);
	}
	if symbol == "gpu_convert" || symbol == "convert" {
		return LoweringAvailability::Unsupported(UnsupportedReason::DynamicFormatConversionPending);
	}
	if !symbol.starts_with("gpu_") {
		return LoweringAvailability::Unsupported(UnsupportedReason::HostBehaviorPending);
	}
	LoweringAvailability::Unsupported(UnsupportedReason::DedicatedPrimitiveCompositionPending)
}

fn dtype_contract(symbol: &str, source: &str, lowering: LoweringAvailability) -> CanonicalDTypeContract {
	if let LoweringAvailability::Scalar(recipe) = lowering {
		return recipe.dtype_contract();
	}
	if let LoweringAvailability::Primitive(recipe) = lowering {
		return recipe.dtype_contract();
	}
	if let LoweringAvailability::Composition(recipe) = lowering {
		return recipe.payload().dtype_contract();
	}
	if matches!(
		lowering,
		LoweringAvailability::Workspace(_) | LoweringAvailability::NonCalculation(_)
	) {
		return CanonicalDTypeContract::NonNumericHostData;
	}
	if let Some(legacy) = explicit_legacy_dtype(symbol, source) {
		return CanonicalDTypeContract::LegacyExcluded {
			legacy,
			canonical_replacement: DType::F32,
		};
	}
	if symbol == "dequant_f32" {
		return CanonicalDTypeContract::F32Payload;
	}
	if symbol.contains("convert") {
		return CanonicalDTypeContract::F32OrI32Payload;
	}
	if !symbol.starts_with("gpu_") {
		return CanonicalDTypeContract::NonNumericHostData;
	}
	if is_i32_payload(symbol) {
		return CanonicalDTypeContract::F32AndI32Payloads;
	}
	if symbol.contains("bernoulli") || symbol.contains("mask") {
		return CanonicalDTypeContract::F32AndI32Payloads;
	}
	CanonicalDTypeContract::F32Payload
}

fn explicit_legacy_dtype(symbol: &str, source: &str) -> Option<LegacyDType> {
	if symbol.contains("_f16") {
		Some(LegacyDType::F16)
	} else if symbol.contains("_f64") || symbol.starts_with("gpu_dgem") || symbol == "gpu_dger_into" || symbol == "gpu_dsyrk" || symbol == "gpu_dasum" || symbol == "gpu_idamax" || symbol == "gpu_scale_f64_inplace" {
		Some(LegacyDType::F64)
	} else if symbol.contains("_u8") {
		Some(LegacyDType::U8)
	} else if source.contains("infer_ops.rs") && symbol == "gpu_convert" {
		Some(LegacyDType::DynamicQuantized)
	} else {
		None
	}
}

fn is_i32_payload(symbol: &str) -> bool {
	[
		"argmax",
		"argmin",
		"argsort",
		"categorical",
		"count",
		"degree",
		"histogram",
		"index",
		"indices",
		"iota",
		"mask",
		"permutation",
		"route",
		"sort",
		"split",
		"topk",
	]
	.iter()
	.any(|needle| symbol.contains(needle))
}

fn family(symbol: &str, source: &str, lowering: LoweringAvailability) -> OperationFamily {
	if let LoweringAvailability::Scalar(_) = lowering {
		return scalar_family(symbol, source);
	}
	if let LoweringAvailability::Primitive(recipe) = lowering {
		return recipe.operation_family();
	}
	if let LoweringAvailability::Composition(recipe) = lowering {
		return recipe.operation_family();
	}
	if let LoweringAvailability::NonCalculation(recipe) = lowering {
		return recipe.operation_family();
	}
	if matches!(lowering, LoweringAvailability::Workspace(_)) {
		return OperationFamily::Workspace;
	}
	if matches!(
		lowering,
		LoweringAvailability::Unsupported(UnsupportedReason::WorkspaceQuery)
	) {
		return OperationFamily::Workspace;
	}
	if matches!(
		lowering,
		LoweringAvailability::Unsupported(UnsupportedReason::LifecycleOperation)
	) {
		return OperationFamily::Lifecycle;
	}
	if matches!(symbol, "Data" | "Model" | "Train" | "Infer") {
		return OperationFamily::Facade;
	}
	if source.contains("safetensors") || symbol.starts_with("parse_") {
		return OperationFamily::Parsing;
	}
	if source.contains("bpe") || source.contains("chat") {
		return OperationFamily::Encoding;
	}
	if source.contains("llm") || source.contains("infer") {
		return OperationFamily::Inference;
	}
	if source.contains("optimizers") {
		return OperationFamily::Optimizer;
	}
	if source.contains("losses") {
		return OperationFamily::Loss;
	}
	if source.contains("attention") || symbol.contains("attention") {
		return OperationFamily::Attention;
	}
	if source.contains("linalg") {
		return linalg_family(symbol);
	}
	if source.contains("reductions") {
		return reduction_family(symbol);
	}
	if source.contains("encoding") {
		return OperationFamily::Encoding;
	}
	if source.contains("graph") {
		return OperationFamily::Graph;
	}
	if source.contains("cluster") {
		return OperationFamily::Clustering;
	}
	if source.contains("bayes") {
		return OperationFamily::Bayesian;
	}
	if source.contains("svm") {
		return OperationFamily::SupportVectorMachine;
	}
	if source.contains("rl") {
		return OperationFamily::ReinforcementLearning;
	}
	if source.contains("forest") || source.contains("catboost") || source.contains("lightgbm") || source.contains("xgboost") {
		return OperationFamily::Tree;
	}
	if source.contains("diffusion") {
		return OperationFamily::Diffusion;
	}
	if source.contains("sequence") {
		return OperationFamily::Sequence;
	}
	if source.contains("moe") {
		return OperationFamily::MixtureOfExperts;
	}
	kernel_family(symbol)
}

fn scalar_family(symbol: &str, source: &str) -> OperationFamily {
	if source.contains("losses.rs") || symbol.contains("_bce_") || symbol.contains("_mse_") || symbol == "gpu_kl_div" {
		OperationFamily::Loss
	} else if source.contains("optimizers.rs") || symbol.contains("_update") {
		OperationFamily::Optimizer
	} else if symbol.contains("dropout") {
		OperationFamily::Random
	} else if [
		"elu", "gelu", "relu", "selu", "sigmoid", "silu", "softplus", "tanh",
	]
	.iter()
	.any(|activation| symbol.contains(activation))
	{
		OperationFamily::Activation
	} else if symbol == "gpu_fill" || symbol == "gpu_fill_f32" {
		OperationFamily::Creation
	} else {
		OperationFamily::Elementwise
	}
}

fn linalg_family(symbol: &str) -> OperationFamily {
	if symbol.contains("fft") {
		OperationFamily::Fft
	} else if symbol.contains("gem") || symbol.contains("ger") || symbol.contains("syrk") || symbol.contains("bmm") {
		OperationFamily::Contraction
	} else {
		OperationFamily::Solver
	}
}

fn reduction_family(symbol: &str) -> OperationFamily {
	if symbol.contains("cum") || symbol.contains("prefix") || symbol.contains("scan") {
		OperationFamily::Scan
	} else if symbol.contains("sort") {
		OperationFamily::ShapeAndIndexing
	} else {
		OperationFamily::Reduction
	}
}

fn kernel_family(symbol: &str) -> OperationFamily {
	if symbol.contains("conv") || symbol.contains("im2col") || symbol.contains("col2im") {
		OperationFamily::Convolution
	} else if symbol.contains("pool") || symbol.contains("upsample") {
		OperationFamily::Pooling
	} else if symbol.contains("norm") {
		OperationFamily::Normalization
	} else if symbol.contains("embed") {
		OperationFamily::Embedding
	} else if symbol.contains("gru") || symbol.contains("lstm") {
		OperationFamily::Recurrent
	} else if symbol.contains("ssm") || symbol.contains("gated_delta") {
		OperationFamily::StateSpace
	} else if symbol.contains("rand") || symbol.contains("dropout") || symbol.contains("bernoulli") {
		OperationFamily::Random
	} else if symbol.contains("reduce") || symbol.contains("sum_all") {
		OperationFamily::Reduction
	} else if symbol.contains("gather") || symbol.contains("scatter") || symbol.contains("slice") || symbol.contains("concat") || symbol.contains("transpose") {
		OperationFamily::ShapeAndIndexing
	} else if symbol.contains("gemm") || symbol.contains("linear") || symbol.contains("matvec") {
		OperationFamily::Contraction
	} else if symbol.contains("loss") || symbol.contains("grad") {
		OperationFamily::Loss
	} else {
		OperationFamily::Other
	}
}

fn alias_contract(symbol: &str) -> AliasContract {
	match symbol {
		"gpu_add_inplace" | "gpu_add_scalar_inplace" | "gpu_mul_inplace" | "gpu_scale_f64_inplace" | "gpu_scale_inplace" | "gpu_sub_inplace" => AliasContract::OutputMustAliasInput { input: 1 },
		"gpu_add_col_scaled_inplace" | "gpu_clip_value" => AliasContract::OutputMustAliasInput { input: 2 },
		"gpu_sgd_update" | "gpu_sgd_update_f32" => AliasContract::OutputMustAliasInput { input: 2 },
		"gpu_scatter_add" | "gpu_softmax_inplace" => AliasContract::OutputMustAliasInput { input: 0 },
		_ if symbol.ends_with("_into") => AliasContract::NoAlias,
		_ if symbol.starts_with("gpu_") => AliasContract::OperationSpecific,
		_ => AliasContract::NoAlias,
	}
}

fn determinism(symbol: &str, lowering: LoweringAvailability) -> DeterminismContract {
	if symbol.contains("rand") || symbol.contains("bernoulli") || symbol.contains("bootstrap") {
		return DeterminismContract::CounterBasedRandom;
	}
	if symbol.contains("scatter") || symbol.contains("histogram") {
		return DeterminismContract::ExplicitAtomicPolicy;
	}
	match lowering {
		LoweringAvailability::Scalar(_) => DeterminismContract::PerElementExactOrder,
		LoweringAvailability::Primitive(_) => DeterminismContract::FixedPrimitiveOrder,
		LoweringAvailability::Composition(_) | LoweringAvailability::Workspace(_) => DeterminismContract::FixedPrimitiveOrder,
		LoweringAvailability::NonCalculation(_) => DeterminismContract::HostDeterministic,
		LoweringAvailability::Unsupported(UnsupportedReason::HostBehaviorPending) => DeterminismContract::HostDeterministic,
		LoweringAvailability::Unsupported(_) => DeterminismContract::PendingDefinition,
	}
}
