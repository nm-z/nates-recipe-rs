#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

//! Canonical inventory and lowering boundary for Recipe's finite public
//! operation surface.
//!
//! Every normative `operation-surface.txt` row remains a distinct
//! descriptor, including duplicate symbols exported by different legacy
//! sources. Each descriptor has an owned scalar program, a concrete
//! primitive program, a finite structured composition, a checked workspace
//! formula, or an explicit non-calculation lifecycle operation. Any future
//! operation without one of those definitions remains unsupported and fails
//! closed.

mod binary_metrics;
mod composition;
mod error;
mod materialize;
mod non_calculation;
mod pooling;
mod primitive;
mod registry;
mod scalar;
mod workspace;

pub use binary_metrics::{
	BinaryClassificationMetricRequest, BinaryMetricMaterialization, BinaryMetricRequirements, MAX_CALIBRATION_BINS,
	MAX_EXACT_BINARY_EXAMPLES, MAX_RECALL_THRESHOLDS, RecallAtOutput, append_binary_classification_metrics,
	binary_metric_requirements, materialize_binary_classification_metrics,
};
pub use composition::{CompositionPayload, CompositionRecipe, CompositionStep, IterationBound, validate_composition};
pub use error::{OperationError, OperationErrorKind, OperationResult};
pub use materialize::{
	IdentityNamespace, MaterializationRequest, MaterializedComposition, MissingConcreteComponent, NamedTensor,
	PreparedParameter, PreparedParameters, RemainingComposition, ResolvedBound, ResolvedComposition,
	ResolvedIteration, ResolvedStep, StageEmission, WorkspaceAllocation, WorkspaceObject, expand_composition,
	materialize_composition, remaining_composition_manifest, validate_identity_namespaces,
};
pub use non_calculation::NonCalculationRecipe;
pub use pooling::{ChannelwiseMaxPool1dPreparation, prepare_channelwise_max_pool_1d};
pub use primitive::{
	AxisRequirement, ContractionClass, PrimitiveFamily, PrimitiveRecipe, PrimitiveRequest, RandomRecipe,
	lower_index_map, lower_primitive,
};
pub use recipe_primitives::LoweredProgram;
pub use registry::{
	AliasContract, CanonicalDTypeContract, DeterminismContract, LegacyDType, LoweringAvailability,
	OperationDescriptor, OperationFamily, OperationId, OperationRegistry, UnsupportedReason,
	channelwise_max_pool_1d_backward_descriptor, channelwise_max_pool_1d_descriptor, operation_registry,
};
pub use scalar::{CompositeScalar, ScalarRecipe, lower_scalar};
pub use workspace::{WorkspaceFormula, WorkspaceUnit, WorkspaceValue, evaluate_workspace};

#[cfg(test)]
mod materialize_attention_tests;
#[cfg(test)]
mod materialize_batch_tests;
#[cfg(test)]
mod materialize_convolution_pooling_tests;
#[cfg(test)]
mod materialize_graph_cluster_rl_tests;
#[cfg(test)]
mod materialize_indexing_sort_encoding_tests;
#[cfg(test)]
mod materialize_loss_metrics_tests;
#[cfg(test)]
mod materialize_tests;
#[cfg(test)]
mod materialize_training_tests;
#[cfg(test)]
mod materialize_tree_boosting_tests;
#[cfg(test)]
mod tests;
