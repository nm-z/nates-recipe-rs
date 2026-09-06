pub use recipe_core::ScalarProgram;
pub use recipe_ops::{CompositionPayload, CompositionRecipe, CompositionStep, IdentityNamespace, IterationBound, LoweredProgram, LoweringAvailability, LoweringHardware, MaterializationRequest, MaterializedComposition, NamedTensor, NonCalculationRecipe, OperationDescriptor, OperationError, OperationErrorKind, OperationRegistry, OperationResult, PreparedParameter, PreparedParameters, PrimitiveRequest, ResolvedBound, ResolvedComposition, ResolvedIteration, ResolvedStep, StageEmission, UnsupportedReason, WorkspaceAllocation, WorkspaceFormula, WorkspaceObject, WorkspaceUnit, WorkspaceValue};

#[must_use]
pub const fn registry() -> OperationRegistry { return recipe_ops::operation_registry(); }

pub fn all() -> impl ExactSizeIterator<Item = OperationDescriptor> { return registry().iter(); }

pub fn resolve(symbol: &str) -> OperationResult<OperationDescriptor> { return registry().resolve_unique(symbol); }

pub fn resolve_exact(symbol: &str, source: &str) -> OperationResult<OperationDescriptor> { return registry().resolve_exact(symbol, source); }

pub fn lower_scalar(descriptor: OperationDescriptor) -> OperationResult<ScalarProgram> { return recipe_ops::lower_scalar(descriptor); }

pub fn lower_primitive(descriptor: OperationDescriptor, request: PrimitiveRequest<'_>, hardware: LoweringHardware) -> OperationResult<LoweredProgram> { return recipe_ops::lower_primitive(descriptor, request, hardware); }

pub fn validate_composition(descriptor: OperationDescriptor) -> OperationResult<()> { return recipe_ops::validate_composition(descriptor); }

pub fn materialize(request: MaterializationRequest<'_>) -> OperationResult<MaterializedComposition> { return recipe_ops::materialize_composition(request); }

pub fn evaluate_workspace(descriptor: OperationDescriptor, dimensions: &[u64]) -> OperationResult<WorkspaceValue> { return recipe_ops::evaluate_workspace(descriptor, dimensions); }
