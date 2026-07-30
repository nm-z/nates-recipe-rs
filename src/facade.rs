use std::cell::RefCell;

pub mod cli {
	include!("cli.rs");
}

/// Immutable user declarations for data, models, and run policy.
///
/// These builders describe intent only. They do not read data, probe hardware,
/// compile kernels, allocate resources, or start execution.
pub mod api {
	include!("api.rs");
}

pub use api::*;

/// Dependency-clean implementation modules available to advanced callers.
pub mod engine {
	pub use recipe_cluster as cluster;
	pub use recipe_core as core;
	pub use recipe_cuda as cuda;
	pub use recipe_executor as executor;
	pub use recipe_host as host;
	pub use recipe_hsa as hsa;
	pub use recipe_ingest as ingest;
	pub use recipe_kernel as kernel;
	pub use recipe_language as language;
	pub use recipe_math as math;
	pub use recipe_native_executor as native_executor;
	pub use recipe_native_probe as native_probe;
	pub use recipe_ogdl as ogdl;
	pub use recipe_ops as ops;
	pub use recipe_planner as planner;
	pub use recipe_prepare as prepare;
	pub use recipe_primitives as primitives;
	pub use recipe_program as program;
	pub use recipe_probe as probe;
	pub use recipe_remote as remote;
	pub use recipe_scheduler as scheduler;
	pub use recipe_text as text;
	pub use recipe_transport as transport;
}

/// Exact operation inventory and honest lowering boundary.
///
/// Every shipped entry is classified as an owned scalar program, direct
/// primitive, finite composition, checked workspace formula, or deterministic
/// host/lifecycle declaration. Future unclassified entries remain visible
/// through [`LoweringAvailability::Unsupported`] and fail closed without
/// substituting legacy behavior.
pub mod operations {
	pub use recipe_core::ScalarProgram;
	pub use recipe_ops::{
		CompositionPayload, CompositionRecipe, CompositionStep, IdentityNamespace, IterationBound, LoweredProgram,
		LoweringAvailability, LoweringHardware, MaterializationRequest, MaterializedComposition, MissingConcreteComponent,
		NamedTensor, NonCalculationRecipe, OperationDescriptor, OperationError, OperationErrorKind,
		OperationRegistry, OperationResult, PreparedParameter, PreparedParameters, PrimitiveRequest,
		RemainingComposition, ResolvedBound, ResolvedComposition, ResolvedIteration, ResolvedStep, StageEmission,
		UnsupportedReason, WorkspaceAllocation, WorkspaceFormula, WorkspaceObject, WorkspaceUnit, WorkspaceValue,
	};

	/// Return the finite normative operation registry.
	#[must_use]
	pub const fn registry() -> OperationRegistry {
		recipe_ops::operation_registry()
	}

	/// Enumerate every source-qualified operation descriptor in normative order.
	pub fn all() -> impl ExactSizeIterator<Item = OperationDescriptor> {
		registry().iter()
	}

	/// Resolve a symbol only when it has exactly one normative entry.
	pub fn resolve(symbol: &str) -> OperationResult<OperationDescriptor> {
		registry().resolve_unique(symbol)
	}

	/// Resolve an operation by its exact public symbol and legacy source.
	pub fn resolve_exact(symbol: &str, source: &str) -> OperationResult<OperationDescriptor> {
		registry().resolve_exact(symbol, source)
	}

	/// Lower a descriptor that owns a scalar elementwise recipe.
	pub fn lower_scalar(descriptor: OperationDescriptor) -> OperationResult<ScalarProgram> {
		recipe_ops::lower_scalar(descriptor)
	}

	/// Verify and lower a descriptor that owns a non-elementwise primitive.
	pub fn lower_primitive(
		descriptor: OperationDescriptor,
		request: PrimitiveRequest<'_>,
		hardware: LoweringHardware,
	) -> OperationResult<LoweredProgram> {
		recipe_ops::lower_primitive(descriptor, request, hardware)
	}

	/// Validate that a structured operation is a finite composition of owned
	/// primitive families. Concrete tensor wiring is materialized during
	/// preparation, after shapes and prepared parameters are fixed.
	pub fn validate_composition(descriptor: OperationDescriptor) -> OperationResult<()> {
		recipe_ops::validate_composition(descriptor)
	}

	/// Materialize a structured operation into a finite validated calculation
	/// graph after shapes and typed preparation facts are fixed.
	pub fn materialize(request: MaterializationRequest<'_>) -> OperationResult<MaterializedComposition> {
		recipe_ops::materialize_composition(request)
	}

	/// Enumerate structured operations whose concrete tensor ABI or formulas
	/// have not yet crossed the fail-closed materialization boundary.
	#[must_use]
	pub fn remaining_compositions() -> Vec<RemainingComposition> {
		recipe_ops::remaining_composition_manifest()
	}

	/// Evaluate an operation's checked static workspace formula.
	pub fn evaluate_workspace(
		descriptor: OperationDescriptor,
		dimensions: &[u64],
	) -> OperationResult<WorkspaceValue> {
		recipe_ops::evaluate_workspace(descriptor, dimensions)
	}
}

pub struct Recipe;

#[expect(
	non_upper_case_globals,
	reason = "preserves the public Recipe builder value"
)]
pub static recipe: Recipe = Recipe;

#[derive(Default)]
struct RecipeSequence {
	data: Option<Data>,
	model: Option<Model>,
}

thread_local! {
	static RECIPE_SEQUENCE: RefCell<RecipeSequence> = RefCell::new(RecipeSequence::default());
}

fn begin_recipe_data(data: Data) {
	RECIPE_SEQUENCE.with(|sequence| {
		let mut sequence = sequence.borrow_mut();
		sequence.data = Some(data);
		sequence.model = None;
	});
}

pub(crate) fn remember_recipe_data(data: Data) {
	begin_recipe_data(data);
}

fn begin_recipe_model(model: Model) {
	RECIPE_SEQUENCE.with(|sequence| sequence.borrow_mut().model = Some(model));
}

pub(crate) fn remember_recipe_model(model: Model) {
	RECIPE_SEQUENCE.with(|sequence| sequence.borrow_mut().model = Some(model));
}

pub(crate) fn take_recipe_sequence() -> Result<(Data, Model), &'static str> {
	take_recipe_sequence_with_diagnostics(
		"recipe.train().run() requires a preceding recipe.data(...) declaration",
		"recipe.train().run() requires a preceding recipe.model() declaration",
	)
}

pub(crate) fn take_recipe_inference_sequence() -> Result<(Data, Model), &'static str> {
	take_recipe_sequence_with_diagnostics(
		"recipe.infer().evaluate() requires a preceding recipe.data(...) declaration",
		"recipe.infer().evaluate() requires a preceding recipe.model() declaration",
	)
}

fn take_recipe_sequence_with_diagnostics(
	missing_data: &'static str,
	missing_model: &'static str,
) -> Result<(Data, Model), &'static str> {
	RECIPE_SEQUENCE.with(|sequence| {
		let mut sequence = sequence.borrow_mut();
		let data = sequence.data.take();
		let model = sequence.model.take();
		let data = data.ok_or(missing_data)?;
		let model = model.ok_or(missing_model)?;
		Ok((data, model))
	})
}

trait IntoDataSources {
	fn into_data_sources(self) -> Vec<String>;
}

impl IntoDataSources for () {
	fn into_data_sources(self) -> Vec<String> {
		Vec::new()
	}
}

impl IntoDataSources for &str {
	fn into_data_sources(self) -> Vec<String> {
		vec![self.to_owned()]
	}
}

impl IntoDataSources for String {
	fn into_data_sources(self) -> Vec<String> {
		vec![self]
	}
}

impl IntoDataSources for &String {
	fn into_data_sources(self) -> Vec<String> {
		vec![self.clone()]
	}
}

impl<S, const N: usize> IntoDataSources for [S; N]
where
	S: AsRef<str>,
{
	fn into_data_sources(self) -> Vec<String> {
		self.into_iter()
			.map(|source| source.as_ref().to_owned())
			.collect()
	}
}

impl<S> IntoDataSources for Vec<S>
where
	S: AsRef<str>,
{
	fn into_data_sources(self) -> Vec<String> {
		self.into_iter()
			.map(|source| source.as_ref().to_owned())
			.collect()
	}
}

impl<S> IntoDataSources for &[S]
where
	S: AsRef<str>,
{
	fn into_data_sources(self) -> Vec<String> {
		self.iter()
			.map(|source| source.as_ref().to_owned())
			.collect()
	}
}

impl Recipe {
	#[allow(private_bounds)]
	pub fn data(&self, sources: impl IntoDataSources) -> Data {
		let mut data = Data::empty();
		for source in sources.into_data_sources() {
			data = data.set(&source);
		}
		begin_recipe_data(data.clone());
		data
	}

	pub fn model(&self) -> Model {
		let model = Model::new();
		begin_recipe_model(model.clone());
		model
	}

	#[must_use]
	pub const fn train(&self) -> Train {
		Train::new()
	}

	#[must_use]
	pub const fn infer(&self) -> Infer {
		Infer::new()
	}
}
