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

mod bayes; mod binary_metrics; mod composition; mod convolution; mod error; mod kmeans; mod knn_outputs;
mod materialize; mod non_calculation; mod pooling; mod primitive; mod registry; mod scalar; mod tree; mod workspace;

pub use bayes::{ CategoricalBayesInferenceMaterialization, CategoricalBayesInferenceRequest,
	CategoricalBayesInferenceRequirements, append_categorical_bayes_inference,
	categorical_bayes_inference_requirements, materialize_categorical_bayes_inference, }; pub use binary_metrics::{
	BinaryClassificationMetricRequest, BinaryMetricMaterialization, BinaryMetricRequirements, MAX_CALIBRATION_BINS,
	MAX_EXACT_BINARY_EXAMPLES, MAX_RECALL_THRESHOLDS, RecallAtOutput,
	binary_metric_requirements, materialize_binary_classification_metrics, };
pub use composition::{CompositionPayload, CompositionRecipe, CompositionStep, IterationBound, validate_composition};
pub use convolution::{ChannelwiseConvolution1dPreparation, prepare_channelwise_convolution_1d};
pub use error::{OperationError, OperationErrorKind, OperationResult}; pub use kmeans::{
	KMeansInitializationMaterialization, KMeansInitializationRequest, KMeansLloydMaterialization, KMeansLloydRequest,
	KMeansRequirements, kmeans_initialization_requirements, kmeans_lloyd_requirements,
	materialize_kmeans_initialization, materialize_kmeans_lloyd_step, }; pub use knn_outputs::{
	KnnAllOutputMaterialization, KnnAllOutputRequest, KnnAllOutputRequirements, KnnOutputRequest,
	KnnOutputRequirement, KnnOutputSpec, append_knn_all_outputs, knn_all_output_requirements, materialize_knn_all_outputs,
}; pub use materialize::{
	IdentityNamespace, MaterializationRequest, MaterializedComposition, NamedTensor, PreparedParameter,
	PreparedParameters, ResolvedBound, ResolvedComposition, ResolvedIteration, ResolvedStep, StageEmission,
	WorkspaceAllocation, WorkspaceObject, expand_composition, materialize_composition, };
pub use non_calculation::NonCalculationRecipe;
pub use pooling::{ChannelwiseMaxPool1dPreparation, prepare_channelwise_max_pool_1d}; pub use primitive::{
	AxisRequirement, ContractionClass, PrimitiveFamily, PrimitiveRecipe, PrimitiveRequest, RandomRecipe,
	lower_index_map, lower_primitive, }; pub use recipe_primitives::{LoweredProgram, LoweringHardware}; pub use registry::{
	AliasContract, CanonicalDTypeContract, DeterminismContract, LegacyDType, LoweringAvailability,
	OperationDescriptor, OperationFamily, OperationId, OperationRegistry, UnsupportedReason, operation_registry, };
pub use scalar::{
	CompositeScalar, FOCAL_ALPHA, FOCAL_GAMMA, ScalarRecipe, canonical_elu_backward_program, canonical_elu_program,
	canonical_focal_with_logits_program, canonical_leaky_relu_backward_program, canonical_leaky_relu_program,
	canonical_prelu_backward_program, canonical_prelu_program, canonical_selu_backward_program,
	canonical_selu_program, lower_scalar, }; pub use tree::{
	TreeEnsembleInferenceMaterialization, TreeEnsembleInferenceRequest, TreeEnsembleInferenceRequirements,
	materialize_tree_ensemble_inference, tree_ensemble_inference_requirements, };
pub use workspace::{WorkspaceFormula, WorkspaceUnit, WorkspaceValue, evaluate_workspace};
